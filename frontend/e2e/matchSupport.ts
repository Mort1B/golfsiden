import { expect, type Browser, type Page } from '@playwright/test'
import { decodeObject, decodeString, decodeUuid } from '../src/api/decoder'
import { decodeAuthSession } from '../src/api/auth'
import { decodeRound, decodeTournament, decodeTournamentRounds } from '../src/api/tournaments/decoders'
import { decodeCard } from '../src/api/matchPlay/cardDecoder'
import { decodeListing } from '../src/api/matchPlay/decoders'
import type { MatchCommand } from '../src/api/matchPlay/contracts'
export async function matchFixture(page: Page, browser: Browser, draft = false, mixed = false) {
  const stamp = `${Date.now()}_${Math.floor(Math.random()*10000)}`, day = new Date(Date.now()+86400000).toISOString().slice(0,10), password = 'match-browser-password'
  const firstName = 'Andreas med et svært langt navn i matchspillturneringen', secondName = 'Bjørn motstander med et annet svært langt navn'
  const created = await page.request.post('/api/onboarding/tournaments', { data: { creator: { account: { username: `match_${stamp}`, password }, player: { display_name: firstName, handicap_index: 0 } }, tournament: { name: `Match ${stamp}`, description: '', start_date: day, end_date: day, counted_rounds: mixed ? 1 : null, mandatory_round_number: null }, rounds: [...(mixed ? [{ round_number: 1, name: 'Slagkonkurranse', round_date: day, scoring_format: 'individual_stroke_play' }] : []), { round_number: mixed ? 2 : 1, name: 'Matchspill med lange navn', round_date: day, scoring_format: 'singles_match_play' }] } })
  expect(created.status(), await created.text()).toBe(201)
  const body = decodeObject(await created.json(),'created'), auth = decodeAuthSession(body.session), tournament = decodeTournament(body.tournament), invitation = decodeObject(body.invitation,'invitation')
  const context = await browser.newContext(); let secondId: string
  try {
    const other = await context.newPage(), response = await other.request.post(`http://127.0.0.1:5173/api/invitations/${decodeUuid(invitation.id,'id')}/register`, { data: { token: decodeString(invitation.token,'token'), account: { username: `opponent_${stamp}`, password }, player: { display_name: secondName, handicap_index: 0 } } })
    expect(response.status(),await response.text()).toBe(201); secondId = decodeUuid(decodeObject(await response.json(),'registered').player_id,'player')
  } finally { await context.close() }
  const rounds = decodeTournamentRounds(await (await page.request.get(`/api/tournaments/${tournament.id}/rounds`)).json(), tournament.id), round = rounds.find(r=>r.scoring_format==='singles_match_play')
  if (!round || !auth.player_id) throw new Error('Missing round/player')
  async function mutate(path: string, data: unknown = {}, method = 'POST') { const response = await page.request.fetch(path, { method, data, headers: { 'x-csrf-token': auth.csrf_token } }); expect(response.ok(), `${path}: ${response.status()} ${await response.text()}`).toBe(true); return response }
  const configured = decodeRound(await (await mutate(`/api/rounds/${round.id}/course-configuration`, { expected_round_updated_at: round.updated_at, selection: { source:'manual',course_name:'Match testbane',location:null,tee:{category:'male',name:'Gul',course_rating:72,slope_rating:113,holes:Array.from({length:18},(_,i)=>({par:4,stroke_index:i+1,distance:null}))} } },'PUT')).json())
  await mutate(`/api/rounds/${round.id}/pairings`, { expected_round_updated_at:configured.updated_at,teams:[],flights:[{id:crypto.randomUUID(),name:'Flight 1',starting_hole:1,tee_time:null,members:[{player_id:auth.player_id},{player_id:secondId}]}],legacy_conversions:[] },'PUT')
  const freshRound = async () => decodeRound(await (await page.request.get(`/api/rounds/${configured.id}`)).json())
  const assign = async () => { const r=await freshRound(); return decodeListing(await (await mutate(`/api/rounds/${r.id}/match-play/matches`,{expected_round_updated_at:r.updated_at,matches:[{first_player_id:auth.player_id,second_player_id:secondId}]},'PUT')).json(),r.id) }
  const start = async () => { const t=decodeTournament(await (await page.request.get(`/api/tournaments/${tournament.id}`)).json()); await mutate(`/api/tournaments/${t.id}/start`,{expected_tournament_updated_at:t.updated_at}); await mutate(`/api/rounds/${configured.id}/open`) }
  let matchId=''; if (!draft) { matchId=(await assign()).matches[0]?.match_id ?? '';await start() }
  const read=async()=>decodeCard(await (await page.request.get(`/api/rounds/${configured.id}/match-play/matches/${matchId}/scoring`)).json(),configured.id,matchId,true)
  const command=async(command:MatchCommand)=>{const card=await read();await mutate(`/api/rounds/${configured.id}/match-play/matches/${matchId}/commands`,{request_id:crypto.randomUUID(),expected_revision:card.revision,command})}
  return { password,secondUsername:`opponent_${stamp}`,auth,tournament,round:configured,firstId:auth.player_id,secondId,firstName,secondName,matchId,read,command,mutate,assign,start,url:`/rounds/${configured.id}/matches/${matchId}/score`,manage:`/manage/tournaments/${tournament.id}?round=${configured.id}#pairings` }
}
export async function noOverflow(page:Page) {
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth)).toBe(true)
  for (const control of await page.locator('.match-panel button:visible, .match-panel summary:visible, .match-actions a:visible, .match-table a:visible').all()) {
    const box = await control.boundingBox(); if (!box) continue
    expect(box.height).toBeGreaterThanOrEqual(44)
    if (await control.isEnabled()) { await control.scrollIntoViewIfNeeded(); await control.click({trial:true}) }
  }
  await page.evaluate(()=>window.scrollTo(0,0))
}
