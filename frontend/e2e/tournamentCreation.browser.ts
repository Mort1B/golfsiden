import { test, expect, type Page } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeTournamentList, decodeTournamentRounds } from '../src/api/tournaments/decoders'
import { decodeCompletionValidation, decodeScoringScorecard, ownerTypeForFormat } from '../src/api/scorecards'
import { LIVE_RESULTS_EXPLANATION } from '../src/features/leaderboards/resultExplanations'
test.skip(process.env.GOLF_TOURNAMENT_CREATION_BROWSER !== '1', 'Requires disposable seed and GOLF_TOURNAMENT_CREATION_BROWSER=1.')
async function login(page: Page, username: string) {
  await page.goto('/login')
  await page.getByLabel('Brukernavn', { exact: true }).fill(username)
  await page.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
  await page.getByRole('button', { name: 'Logg inn', exact: true }).click()
  await expect(page).not.toHaveURL(/\/login/)
}
async function layout(page: Page, state: string, selector = '.onboarding-shell') {
  const panel = page.locator(selector)
  for (const width of [320,390,1280]) {
    await page.setViewportSize({ width, height:900 })
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    for (const button of await panel.locator('button:visible').all()) {
      expect((await button.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      if (await button.isEnabled()) await button.click({ trial:true })
    }
    await panel.screenshot({ path:`/tmp/golf-creation-${state}-${width}.png` })
  }
}
async function review(page: Page, name: string) {
  await page.goto('/tournaments')
  await page.getByRole('link', { name:'Opprett ny turnering' }).click()
  await page.getByLabel('Turneringsnavn', { exact:true }).fill(name)
  await page.getByRole('button', { name:'Neste', exact:true }).click()
  await page.getByRole('button', { name:'Neste', exact:true }).click()
  await expect(page.getByRole('heading', { name:'Kontroller opplysningene' })).toBeVisible()
  await expect(page.locator('input[type=password]')).toHaveCount(0)
}
test('ordinary signed-in player creates multiple independent tournaments and safely recovers a lost response', async ({ page }) => {
  await login(page,'anders')
  const originalAuth = decodeAuthSession(await (await page.request.get('/api/auth/session')).json())
  const errors: string[] = []; const failed: string[] = []
  page.on('pageerror', (e)=>errors.push(e.message))
  page.on('console', (m)=>{if(m.type()==='error')errors.push(m.text())})
  page.on('response',(r)=>{if(r.status()>=400)failed.push(String(r.status()))})
  const original = decodeTournamentList(await (await page.request.get('/api/tournaments')).json())
  const ids: string[] = []
  for (const [i,name] of ['Testtur før den virkelige turen', 'Den virkelige golfturen med et langt og tydelig turneringsnavn'].map(name => `${name} ${Date.now()}`).entries()) {
    await review(page,name)
    if(i===0) await layout(page,'review')
    const response=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/tournaments' && r.request().method()==='POST')
    await page.getByRole('button',{name:'Opprett turnering',exact:true}).click()
    const r=await response;expect(r.status()).toBe(201);expect(r.headers()['set-cookie']).toBeUndefined()
    const receipt=await r.json();ids.push(receipt.tournament_id)
    await expect(page).toHaveURL(new RegExp('/manage/tournaments/'+receipt.tournament_id))
    await expect(page.getByRole('heading',{name,exact:true})).toBeVisible()
    await expect(page.locator('#entrants').getByText('Anders',{exact:true})).toBeVisible()
  }
  expect(new Set(ids).size).toBe(2)
  await page.goto('/tournaments')
  for(const trip of original) await expect(page.getByRole('heading',{name:trip.name,exact:true})).toBeVisible()
  await layout(page,'list','.tournament-list-page')
  const currentAuth=decodeAuthSession(await(await page.request.get('/api/auth/session')).json())
  expect(currentAuth).toEqual(originalAuth)
  expect(errors).toEqual([]);expect(failed).toEqual([])
  // Inject loss only after real commit, then throttle a retry, then replay normally.
  let calls=0;const keys: string[]=[]
  await page.route('**/api/tournaments',async route=>{
    if(route.request().method()!=='POST')return route.continue()
    calls++;keys.push(route.request().postDataJSON().request_id)
    if(calls===1){const committed=await route.fetch();expect(committed.status()).toBe(201);return route.abort('failed')}
    if(calls===2)return route.fulfill({status:429,json:{error:{code:'rate_limited',message:'retry later'}}})
    return route.continue()
  })
  await review(page,`Turnering med mistet svar ${Date.now()}`)
  await page.getByRole('button',{name:'Opprett turnering',exact:true}).click()
  await expect(page.getByRole('alert')).toContainText('Vi fikk ikke bekreftet')
  await expect(page.getByRole('button',{name:'Tilbake',exact:true})).toBeDisabled()
  await layout(page,'uncertain')
  await page.getByRole('button',{name:'Opprett turnering',exact:true}).click()
  await expect(page.getByRole('alert')).toContainText('For mange opprettingsforsøk')
  await expect(page.getByRole('button',{name:'Tilbake',exact:true})).toBeDisabled()
  await page.getByRole('button',{name:'Opprett turnering',exact:true}).click()
  await expect(page).toHaveURL(/\/manage\/tournaments\//)
  expect(keys).toHaveLength(3);expect(new Set(keys).size).toBe(1)
  const final=decodeTournamentList(await(await page.request.get('/api/tournaments')).json())
  expect(final.length).toBe(original.length+3)
})
test('live standings, card completeness and completion explanation use consistent wording',async({page})=>{
  await login(page,'admin')
  const auth=decodeAuthSession(await(await page.request.get('/api/auth/session')).json())
  const trip=decodeTournamentList(await(await page.request.get('/api/tournaments')).json()).find(t=>t.name==='Guttas Golf 2026')
  if(!trip)throw new Error('Missing seed')
  const rounds=decodeTournamentRounds(await(await page.request.get(`/api/tournaments/${trip.id}/rounds`)).json(),trip.id)
  const round=rounds[0];if(!round)throw new Error('Missing round')
  async function post(path:string,data:unknown={},method='POST'){
    const r=await page.request.fetch(path,{method,data,headers:{'x-csrf-token':auth.csrf_token}});expect(r.ok(),path+' '+r.status()).toBe(true)
  }
  if(trip.status==='draft') await post(`/api/tournaments/${trip.id}/start`,{expected_tournament_updated_at:trip.updated_at})
  if(round.status==='draft') await post(`/api/rounds/${round.id}/open`)
  const progress=decodeCompletionValidation(await(await page.request.get(`/api/rounds/${round.id}/completion-validation`)).json(),round.id,ownerTypeForFormat(round.scoring_format))
  for(const {owner} of progress.owners){
    const path=`/api/rounds/${round.id}/scorecards/${owner.type}/${owner.id}`
    const card=decodeScoringScorecard(await(await page.request.get(path+'/scoring')).json(),round.id,owner)
    const firstHole=card.holes[0]
    if(!firstHole)throw new Error('Missing hole')
    await post(`/api/rounds/${round.id}/scores`,{owner,hole_id:firstHole.hole_id,gross_strokes:firstHole.par+1},'PUT')
    for(const hole of card.holes)await post(`/api/rounds/${round.id}/scores`,{owner,hole_id:hole.hole_id,gross_strokes:hole.par},'PUT')
  }
  await page.goto(`/leaderboard?tournament=${trip.id}&scope=round&round=${round.id}&metric=net`)
  await expect(page.getByText('Alle hull ført · venter på bekreftelse').first()).toBeVisible()
  await layout(page,'cards','.standings-section')
  await page.goto(`/leaderboard?tournament=${trip.id}&scope=tournament&round=${round.id}&metric=net`)
  await expect(page.getByText(LIVE_RESULTS_EXPLANATION,{exact:true})).toBeVisible()
  await expect(page.getByText(/Kvalifisering: 0 av 3 nødvendige fullførte runder/).first()).toBeVisible()
  await layout(page,'standings','.standings-section')
  await page.locator('.leaderboard-row-link').first().click()
  await expect(page.getByText(LIVE_RESULTS_EXPLANATION,{exact:true})).toBeVisible()
  await expect(page.getByText(/Med i vist sammenlagtresultat/)).toBeVisible()
  await layout(page,'history','.player-history')
  for(const {owner} of progress.owners)await post(`/api/rounds/${round.id}/scorecards/${owner.type}/${owner.id}/confirm`)
  await page.goto(`/manage/tournaments/${trip.id}?round=${round.id}#lifecycle`)
  await page.getByRole('button',{name:'Fullfør runden',exact:true}).click()
  await expect(page.getByText(/Synlig score fra den åpne runden kan allerede inngå foreløpig/)).toBeVisible()
  await layout(page,'completion','.round-lifecycle-detail')
})
