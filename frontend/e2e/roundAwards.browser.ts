import { expect, test, type Page } from '@playwright/test'
import { at, claimManager, releaseFixture, scoreTeam, type ReleaseFixture } from './fantasyReleaseSupport'
import { stablefordFixture } from './stablefordSupport'
import { decodeArray, decodeObject, decodeUuid } from '../src/api/decoder'
import { decodeRoundLeaderboard, decodeTournamentLeaderboard } from '../src/api/leaderboards'
import { decodeStablefordRead } from '../src/api/stableford/decoders'
import { decodeAwardsView } from '../src/api/roundAwards/decode'

test.skip(process.env.GOLF_AWARDS_BROWSER !== '1','Requires disposable local API and Chrome.')
async function live(page:Page,url:string){
 await page.addInitScript(()=>{const Original=window.EventSource;window.EventSource=class extends Original{constructor(url:string|URL,options?:EventSourceInit){super(url,options);this.addEventListener('open',()=>{document.documentElement.dataset.awardsLive='true'})}}})
 await page.goto(url);await expect(page.locator('html')).toHaveAttribute('data-awards-live','true',{timeout:30000})
}
async function layouts(page:Page,name:string,selector='.round-awards'){
 for(const width of [320,390,1280]){await page.setViewportSize({width,height:900});const region=page.locator(selector);await region.scrollIntoViewIfNeeded();expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);for(const control of await region.locator('button, select, textarea').all()){if(await control.isVisible()){const box=await control.boundingBox();expect(box?.height).toBeGreaterThanOrEqual(44)}}await page.screenshot({path:`/tmp/golf-awards-${name}-${width}.png`,fullPage:true});await region.screenshot({path:`/tmp/golf-awards-${name}-${width}-region.png`})}
}
function evidence(page:Page){const errors:string[]=[],failures:string[]=[];page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error'&&!m.text().includes('Failed to load resource'))errors.push(m.text())});page.on('requestfailed',r=>{if(!r.url().includes('/live')&&!r.failure()?.errorText.includes('ERR_ABORTED'))failures.push(new URL(r.url()).pathname)});return {errors,failures}}
async function award(page:Page,type:string,player:string,correction=false){const group=page.getByRole('group',{name:`${type} · valgfri −1`});await group.getByLabel(`Vinner av ${type}`).selectOption(player);await group.getByLabel(`Begrunnelse for ${type}`).fill('Målt og bekreftet etter runden');if(correction)await group.getByRole('checkbox').check();await group.getByRole('button',{name:`Lagre ${type}`}).click();await expect(group.getByText('Innsendingen er bekreftet. Gjeldende utmerkelser er oppdatert.')).toBeVisible()}
async function scorePlayer(f:ReleaseFixture,round:string,player:string){const path=`/api/rounds/${round}/scorecards/player/${player}`;const card=decodeObject(await(await f.page.request.get(`${path}/scoring`)).json(),'card');const holes=decodeArray(card.holes,'holes',(v,p)=>decodeUuid(decodeObject(v,p).hole_id,p));for(const hole_id of holes)await f.mutate(`/api/rounds/${round}/scores`,{owner:{type:'player',id:player},hole_id,gross_strokes:4},'PUT');await f.mutate(`${path}/confirm`)}
for(const team of [false,true])test(`optional awards preserve ${team?'shared team':'individual'} base scores, Fantasy and hidden-member privacy`,async({page,browser})=>{
 const observed=evidence(page),f=await releaseFixture(page,[team?'team_scramble':'individual_stroke_play'],[0,0,0,0]),round=at(f.rounds,0),other=await claimManager(f,browser,1)
 try{
 const pairs=await f.configure(0,[0,1,2,3],team)
 await f.mutate(`${f.base}/rounds/${round.id}/lineup`,{request_id:crypto.randomUUID(),expected_revision:0,picks:f.players,captain:at(f.players,0)},'PUT')
 await f.start();await f.mutate(`/api/rounds/${round.id}/open`)
 if(team)for(const pair of pairs)await scoreTeam(f,0,pair.id,4);else for(const player of f.players)await scorePlayer(f,round.id,player)
 await f.mutate(`/api/rounds/${round.id}/complete`)
 const before=await f.board(0),totalsBefore=await f.totals()
 const boardPath=`/api/rounds/${round.id}/leaderboards/net`
 const hiddenBefore=await(await other.page.request.get(boardPath)).json()
 await live(page,`/manage/tournaments/${f.tournament.id}?round=${round.id}#lifecycle`)
 await expect(page.getByLabel('Vinner av Closest to pin')).toHaveValue('')
 await layouts(page,team?'team-empty':'individual-empty')
 await award(page,'Closest to pin',at(f.players,0))
 await award(page,'Longest drive',at(f.players,0))
 await layouts(page,team?'team-both':'individual-both')
 expect(await f.board(0)).toEqual(before);expect(await f.totals()).toEqual(totalsBefore)
 expect(await(await other.page.request.get(boardPath)).json()).toEqual(hiddenBefore)
 expect((await other.page.request.get(`/api/rounds/${round.id}/awards`)).status()).toBe(403)
 const board=decodeRoundLeaderboard(await(await page.request.get(boardPath)).json(),round.id,f.tournament.id,'net')
 if(team){expect(board.entries[0]?.net_total).toBe(72);expect(board.entries[0]?.members.find(m=>m.player_id===at(f.players,0))?.net_awards?.adjustment).toBe(-2);expect(board.entries[0]?.members.find(m=>m.player_id===at(f.players,1))?.net_awards).toBeUndefined()}
 else expect(board.entries[0]?.net_total).toBe(70)
 const overall=decodeTournamentLeaderboard(await(await page.request.get(`/api/tournaments/${f.tournament.id}/leaderboards/net`)).json(),f.tournament.id,'net');expect(overall.entries.find(e=>e.player_id===at(f.players,0))?.net_total).toBe(70);expect(overall.entries.find(e=>e.player_id===at(f.players,1))?.net_total).toBe(72)
 await page.goto(`/tournaments/${f.tournament.id}/results/players/${at(f.players,0)}?metric=net`);await expect(page.getByText(/Netto før utmerkelser: 72/)).toBeVisible();await layouts(page,team?'team-history':'individual-history','.player-history')
 await page.goto(`/leaderboard?tournament=${f.tournament.id}&scope=round&round=${round.id}&metric=net`);await expect(page.getByText(/Justert netto: 70/)).toBeVisible();await layouts(page,team?'team-result':'individual-result','.standings-section')
 const visibility=decodeObject(await(await page.request.get(`/api/tournaments/${f.tournament.id}/final-round-visibility`)).json(),'visibility')
 await f.mutate(`/api/tournaments/${f.tournament.id}/final-round-visibility`,{back_nine_hidden:false,expected_visibility_updated_at:visibility.visibility_updated_at},'PATCH')
 await other.page.goto(`/tournaments/${f.tournament.id}/results/players/${at(f.players,0)}?metric=net`);await expect(other.page.getByText(/Justert netto: 70/)).toBeVisible();expect(await other.page.getByRole('button',{name:'Lagre Closest to pin'}).count()).toBe(0)
 await page.goto(`/manage/tournaments/${f.tournament.id}?round=${round.id}#lifecycle`)
 await award(page,'Closest to pin',at(f.players,1),true);await award(page,'Longest drive','',true)
 const final=decodeAwardsView(await(await page.request.get(`/api/rounds/${round.id}/awards`)).json(),round.id,f.tournament.id);expect(final.history).toHaveLength(4);expect(final.awards.map(a=>a.player_id)).toEqual([at(f.players,1),null])
 expect(observed.errors).toEqual([]);expect(observed.failures).toEqual([])
 }finally{await other.context.close()}
})
test('locked Stableford shows native points and separate competition points',async({page})=>{
 const observed=evidence(page),f=await stablefordFixture(page)
 for(let hole=1;hole<=18;hole++)await f.save(hole,{type:'numeric',gross_strokes:4})
 await f.mutate(`${f.cardPath}/confirm`);await f.mutate(`/api/rounds/${f.round.id}/complete`);await f.mutate(`/api/rounds/${f.round.id}/lock`)
 const read=async()=>decodeStablefordRead(await(await page.request.get(f.cardPath)).json(),f.round.id,f.owner.id)
 const native=await read()
 await live(page,`/manage/tournaments/${f.tournament.id}?round=${f.round.id}#lifecycle`)
 await award(page,'Closest to pin',f.owner.id,true);await award(page,'Longest drive',f.owner.id,true)
 expect((await read()).values).toEqual(native.values)
 await page.goto(`/leaderboard?tournament=${f.tournament.id}&scope=round&round=${f.round.id}&metric=net`)
 await expect(page.getByText(/Konkurransepoeng: 56/)).toBeVisible();await expect(page.getByText('56 poeng',{exact:true})).toBeVisible();await layouts(page,'stableford-result','.standings-section')
 await page.goto(`/tournaments/${f.tournament.id}/results/players/${f.owner.id}?metric=net`);await expect(page.getByText(/Native nettopoeng: 54/)).toBeVisible();await layouts(page,'stableford-history','.player-history')
 expect(observed.errors).toEqual([]);expect(observed.failures).toEqual([])
})
test('lost award response survives failed reads and replays exactly; acknowledged retry reads only',async({page})=>{
 const observed=evidence(page),f=await stablefordFixture(page)
 for(let hole=1;hole<=18;hole++)await f.save(hole,{type:'numeric',gross_strokes:4})
 await f.mutate(`${f.cardPath}/confirm`);await f.mutate(`/api/rounds/${f.round.id}/complete`)
 await live(page,`/manage/tournaments/${f.tournament.id}?round=${f.round.id}#lifecycle`)
 const path=`/api/rounds/${f.round.id}/awards`,writes:unknown[]=[];let lose=true,failReads=false
 await page.route(`**${path}`,async route=>{if(route.request().method()==='PUT'){writes.push(route.request().postDataJSON());if(lose){lose=false;const response=await route.fetch();expect(response.ok()).toBe(true);failReads=true;await route.abort('connectionfailed')}else await route.continue()}else if(failReads)await route.fulfill({status:503,contentType:'application/json',body:JSON.stringify({error:{code:'test_unavailable',message:'Midlertidig lesefeil'}})});else await route.continue()})
 const group=page.getByRole('group',{name:'Closest to pin · valgfri −1'});await group.getByLabel('Vinner av Closest to pin').selectOption(f.owner.id);await group.getByLabel('Begrunnelse for Closest to pin').fill('Målt');await group.getByRole('button',{name:'Lagre Closest to pin'}).click()
 await expect(page.getByText('Midlertidig lesefeil')).toBeVisible();await layouts(page,'uncertain-error');await layouts(page,'uncertain-recovery','.award-recovery')
 await page.getByRole('button',{name:'Avklar samme Closest to pin-innsending'}).click();await expect(page.getByRole('button',{name:'Prøv oppdatering igjen'})).toBeVisible();expect(writes).toHaveLength(2);expect(writes[0]).toEqual(writes[1])
 failReads=false;await page.getByRole('button',{name:'Prøv oppdatering igjen'}).click();await expect(page.getByText('Innsendingen er bekreftet. Gjeldende utmerkelser er oppdatert.')).toBeVisible();expect(writes).toHaveLength(2)
 await layouts(page,'recovered')
 expect(observed.errors).toEqual([]);expect(observed.failures).toEqual([path])
})
