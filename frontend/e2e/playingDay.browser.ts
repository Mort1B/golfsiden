import { expect, test, type Page } from '@playwright/test'
import { offlineFixture, offlineEvents, offlineLayout, queueRows } from './offlineSupport'
import { fourBallFixture } from './fourBallSupport'
import { stablefordFixture } from './stablefordSupport'
import { fantasyFixture, fantasyLayouts } from './fantasySupport'

test.skip(process.env.GOLF_PLAYING_DAY_BROWSER !== '1', 'Requires disposable local API/PostgreSQL and Chrome.')
function clean(events:ReturnType<typeof offlineEvents>) {
  expect(events.errors).toEqual([])
  expect(events.statuses.filter(s=>![401,409,503].includes(s))).toEqual([])
  expect(events.failures.every(s=>/ERR_INTERNET_DISCONNECTED|ERR_NETWORK_CHANGED|ERR_ABORTED|ERR_FAILED|aborted/i.test(s))).toBe(true)
}
async function review(page:Page,name:string) {
  const button=page.getByRole('button',{name,exact:true})
  await expect(button).toBeEnabled();await button.focus();await page.keyboard.press('Enter')
  await expect(page.getByRole('region',{name:'Fremdrift på scorekortet'})).toBeFocused()
  await expect(page).toHaveURL(/view=summary/)
  await page.keyboard.press('Tab');await expect(page.getByRole('combobox',{name:'Hull',exact:true})).toBeFocused()
  expect(await page.evaluate(()=>{
    const controls=document.querySelector('.score-hole-controls'),summary=document.querySelector('.scorecard-summary, .four-ball-summary, .stableford-summary')
    return !!controls&&!!summary&&!!(controls.compareDocumentPosition(summary)&Node.DOCUMENT_POSITION_FOLLOWING)
  })).toBe(true)
}
for(const format of ['stroke','team','four-ball','stableford'] as const)test(`${format}: local distinct progress, keyboard review and server-only confirmation`,async({page,context,browser})=>{
  const f=format==='four-ball'?await fourBallFixture(page,browser):format==='stableford'?await stablefordFixture(page):await offlineFixture(page,format==='team'?'team_scramble':'individual_stroke_play')
  const events=offlineEvents(page),dedicated=format==='four-ball'||format==='stableford'
  await page.goto(f.url(18));await expect(page.getByRole('button',{name:'Kontroller manglende hull'})).toBeEnabled()
  await expect(page.getByRole('button',{name:'Se over scorekortet'})).toHaveCount(0)
  await page.goto(f.url(1));await expect(page.getByRole('button',{name:/Registrer par/}).first()).toBeEnabled()
  await context.setOffline(true)
  const add=page.getByRole('button',{name:dedicated?/Ett slag mer for/: 'Legg til ett slag',exact:!dedicated}).first()
  await page.getByRole('button',{name:/Registrer par/}).first().click();await expect.poll(async()=>(await queueRows(page)).length).toBe(1)
  await add.click();await expect(page.locator('.card-review')).toContainText('1 av 18 hull ført på kortet')
  await expect(page.locator('.card-review')).toContainText('0 av 18 hull kontrollert på serveren')
  await page.getByRole('button',{name:dedicated?'Neste hull':'Neste',exact:true}).click()
  await page.getByRole('button',{name:/Registrer par/}).first().click();await expect.poll(async()=>(await queueRows(page)).length).toBe(2)
  await expect(page.locator('.card-review')).toContainText('2 av 18 hull ført på kortet')
  await expect(page.locator('.card-review')).toContainText('2 hull har lokale endringer')
  await offlineLayout(page,`playing-${format}-pending`,'.score-page')
  await review(page,'Kontroller manglende hull')
  for(const confirm of await page.getByRole('button',{name:/^Bekreft/}).all())await expect(confirm).toBeDisabled()
  await offlineLayout(page,`playing-${format}-review`,'.score-page')
  await context.setOffline(false);await expect.poll(async()=>(await queueRows(page)).length,{timeout:35000}).toBe(0)
  await expect(page.locator('.card-review')).toContainText('2 av 18 hull kontrollert på serveren')
  // Complete the actual server card, then review from hole 4 rather than hole 18.
  for(let h=1;h<=18;h++){
    if('firstId'in f)await f.save(h,f.firstId,{type:'numeric',gross_strokes:4})
    else if('cardPath'in f)await f.save(h,{type:'numeric',gross_strokes:4})
    else await f.save(h,4)
  }
  await page.goto(f.url(4));await review(page,'Se over scorekortet')
  expect((await f.read()).confirmed).toBe(false)
  const confirm=page.getByRole('button',{name:format==='four-ball'?'Bekreft lagets scorekort':format==='stableford'?'Bekreft scorekort':'Bekreft fullført scorekort',exact:true})
  if(format==='four-ball'){
    await expect(confirm).toBeDisabled()
    await page.getByLabel('Jeg bekrefter at tomme partnerfelt ikke bidrar med score.').check()
  }
  await expect(confirm).toBeEnabled();await confirm.focus();await page.keyboard.press('Enter')
  await expect.poll(async()=>(await f.read()).confirmed).toBe(true)
  clean(events)
})

test('real writable cards switch above summary without changing the selected hole',async({page})=>{
  const f=await fantasyFixture(page),r=f.rounds[0];if(!r)throw new Error('round')
  await f.prepare();await f.mutate(`/api/rounds/${r.id}/open`)
  const events=offlineEvents(page)
  await page.goto(`/score?tournament=${f.tournament.id}&round=${r.id}&hole=7&view=summary`)
  const rail=page.getByRole('navigation',{name:'Bytt mellom scorekort du kan føre'})
  await expect(rail.getByRole('button')).toHaveCount(4)
  expect(await rail.evaluate(node=>{const summary=document.querySelector('.scorecard-summary');return !!summary&&!!(node.compareDocumentPosition(summary)&Node.DOCUMENT_POSITION_FOLLOWING)})).toBe(true)
  await rail.getByRole('button').nth(1).focus();await page.keyboard.press('Enter')
  await expect(page).toHaveURL(new RegExp(`owner=${f.players[1]}`));await expect(page).toHaveURL(/hole=7/)
  await expect(rail.getByRole('button').nth(1)).toHaveAttribute('aria-pressed','true')
  await offlineLayout(page,'playing-quick-cards','.score-page')
  clean(events)
})

test('Fantasy compact editing and keyboard sections retain draft and expose uncertainty',async({page})=>{
  const f=await fantasyFixture(page),events=offlineEvents(page)
  await f.mutate(`/api/tournaments/${f.tournament.id}/fantasy`,{enabled:true},'PUT');await f.mutate(`/api/tournaments/${f.tournament.id}/fantasy/entry`)
  await page.goto(f.url)
  for(const name of f.names)await page.getByRole('checkbox',{name,exact:true}).check()
  await page.getByLabel('Kaptein · doble poeng').selectOption(f.players[0]??'')
  await page.getByRole('button',{name:'Lagre firer og kaptein'}).click()
  await expect(page.getByText('Lagret lag · revisjon 1')).toBeVisible()
  // Live invalidation can overlap the read check; an acknowledged receipt offers reads only.
  await expect(page.getByText(/^Handlingen er bekreftet/)).toBeVisible()
  const retry=page.getByRole('button',{name:'Prøv oppdatering igjen',exact:true})
  if(await retry.isVisible())await retry.click()
  await expect(page.getByRole('button',{name:'Endre valg'})).toBeEnabled()
  await expect(page.getByRole('checkbox')).toHaveCount(0);await fantasyLayouts(page,'playing-compact')
  await page.getByRole('button',{name:'Endre valg'}).focus();await page.keyboard.press('Enter');await expect(page.locator('.fantasy-page fieldset')).toBeFocused()
  await page.getByLabel('Kaptein · doble poeng').selectOption(f.players[1]??'')
  await page.getByRole('button',{name:'Min firer',exact:true}).focus();await page.keyboard.press('Tab')
  await expect(page.getByRole('button',{name:'Poengtavler',exact:true})).toBeFocused();await page.keyboard.press('Enter')
  await expect(page.getByRole('checkbox')).toHaveCount(0)
  await page.keyboard.press('Shift+Tab');await expect(page.getByRole('button',{name:'Min firer',exact:true})).toBeFocused();await page.keyboard.press('Enter')
  await expect(page.getByLabel('Kaptein · doble poeng')).toHaveValue(f.players[1]??'')
  await page.route('**/fantasy/rounds/*/lineup',route=>route.abort('failed'))
  await page.getByRole('button',{name:'Lagre firer og kaptein'}).click();await expect(page.getByText(/Lagringen er ikke bekreftet/)).toBeVisible()
  await page.getByRole('button',{name:'Poengtavler',exact:true}).click()
  await expect(page.getByRole('button',{name:'Se innsendingen i Min firer'})).toBeVisible();await fantasyLayouts(page,'playing-warning-on-boards')
  await page.getByRole('button',{name:'Se innsendingen i Min firer'}).click();await expect(page.getByRole('button',{name:'Avklar samme innsending'})).toBeVisible()
  clean(events)
})

test('overview loading, error retry, empty, populated and fresh continuation',async({page})=>{
  const f=await offlineFixture(page),events=offlineEvents(page),path=`**/api/tournaments/${f.tournament.id}/rounds`
  let release=()=>{},fail=true
  const gate=new Promise<void>(resolve=>{release=resolve})
  await page.route(path,async route=>{await gate;if(fail)return route.fulfill({status:503,json:{error:{code:'unavailable',message:'internal transport detail'}}});return route.continue()})
  await page.goto(`/tournaments/${f.tournament.id}`);await expect(page.getByText('Laster …')).toBeVisible();await offlineLayout(page,'playing-overview-loading','.tournament-overview')
  release();await expect(page.getByText('Kunne ikke hente rundene. Kontroller forbindelsen og prøv igjen.')).toBeVisible();await expect(page.getByText('internal transport detail')).toHaveCount(0)
  await offlineLayout(page,'playing-overview-error','.tournament-overview');fail=false
  await page.getByRole('button',{name:'Prøv igjen'}).focus();await page.keyboard.press('Enter')
  await expect(page.getByRole('link',{name:/Fortsett scoreføring/})).toBeVisible();await offlineLayout(page,'playing-overview-populated','.tournament-overview')
  await page.getByRole('link',{name:/Fortsett scoreføring/}).focus();await page.keyboard.press('Enter');await expect(page).toHaveURL(new RegExp(`round=${f.round.id}.*resume=1`));await expect(page.getByRole('button',{name:/Registrer par/})).toBeEnabled()
  await page.unroute(path);await page.route(path,route=>route.fulfill({json:[]}));await page.goto(`/tournaments/${f.tournament.id}`)
  await expect(page.getByText(/Ingen runder er opprettet ennå/)).toBeVisible();await expect(page.getByRole('link',{name:/Fortsett scoreføring/})).toHaveCount(0);await offlineLayout(page,'playing-overview-empty','.tournament-overview')
  clean(events)
})
