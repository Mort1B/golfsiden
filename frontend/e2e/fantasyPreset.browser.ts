import { expect, test } from '@playwright/test'
import { at, browserEvidence, claimManager, releaseFixture } from './fantasyReleaseSupport'
import { fantasyLayouts } from './fantasySupport'
import { observeFantasyReads } from './fantasyReadEvidence'
import { roundView } from '../src/api/fantasy/selections'

async function observeLive(page: import('@playwright/test').Page) {
 await page.addInitScript(()=>{
  const Original=window.EventSource
  window.EventSource=class extends Original {
   constructor(url:string|URL,options?:EventSourceInit){super(url,options);this.addEventListener('open',()=>{document.documentElement.dataset.fantasyLiveReady='true'})}
  }
 })
}
async function waitForLive(page: import('@playwright/test').Page) {
 await expect(page.locator('html')).toHaveAttribute('data-fantasy-live-ready','true',{timeout:30000})
 await expect(page.getByRole('heading',{name:'Planlegg alle runder'})).toBeVisible()
}
test.skip(process.env.GOLF_FANTASY_BROWSER !== '1', 'Requires disposable local API and Chrome.')
test('presets three rounds before sporting teams, overrides one captain, and preserves private picks',async({page,browser})=>{
 await observeLive(page)
 const evidence=browserEvidence(page)
 const f=await releaseFixture(page,['team_scramble','two_player_foursomes','team_scramble'],[0,0,0,0,0,0])
 const other=await claimManager(f,browser,5)
 try {
  await page.goto(f.url);await waitForLive(page)
  await expect(page.locator('.fantasy-presets > li')).toHaveCount(3)
  for(const name of f.names.slice(0,4))await page.getByRole('checkbox',{name,exact:true}).check()
  await page.getByLabel('Kaptein · doble poeng').selectOption(at(f.players,0))
  const copy=page.getByRole('button',{name:'Kopier fireren til andre åpne runder'})
  await copy.focus();await page.keyboard.press('Enter')
  await expect(page.getByRole('region',{name:'Gjennomgå kopiering'})).toBeFocused()
  await page.keyboard.press('Escape');await expect(copy).toBeFocused()
  await copy.click();await page.getByRole('button',{name:'Forbered valgte runder'}).click()
  await page.getByLabel('Runde for Min firer').selectOption(at(f.rounds,1).id)
  await page.getByLabel('Kaptein · doble poeng').selectOption(at(f.players,1))
  await fantasyLayouts(page,'preset-reviewed')
  await page.getByRole('button',{name:'Lagre gjennomgåtte utkast (3)'}).click()
  await expect(page.getByText(/3 av 3 innsendinger bekreftet/)).toContainText('Alle gjennomgåtte utkast er lagret.')
  for(const [index,round] of f.rounds.entries()){
   const response=await page.request.get(`${f.base}/rounds/${round.id}`)
   const own=roundView(await response.json(),round.id,f.auth.user_id)
   expect(own.selections.find(s=>s.user_id===f.auth.user_id)?.receipt?.captain).toBe(at(f.players,index===1?1:0))
   const privateResponse=await other.page.request.get(`${f.base}/rounds/${round.id}`)
   const privateView=roundView(await privateResponse.json(),round.id,other.auth.user_id)
   expect(privateView.selections.some(s=>s.user_id===f.auth.user_id)).toBe(false)
  }
  await fantasyLayouts(page,'preset-saved');await evidence()
 }finally{await other.context.close()}
})
test('pauses partial saving after a lost response and resumes without resending acknowledged rounds',async({page})=>{
 await observeLive(page)
 const errors:string[]=[],failed:string[]=[],writes:{round:string;body:unknown}[]=[]
 page.on('pageerror',error=>errors.push(error.message))
 page.on('console',message=>{if(message.type()==='error'&&!message.text().includes('Failed to load resource'))errors.push(message.text())})
 page.on('requestfailed',request=>{if(!request.url().includes('/live')&&!request.failure()?.errorText.includes('ERR_ABORTED'))failed.push(new URL(request.url()).pathname)})
 const f=await releaseFixture(page,['individual_stroke_play','individual_stroke_play','individual_stroke_play'],[0,0,0,0])
 let lose=true
 await page.route('**/fantasy/rounds/*/lineup',async route=>{
  const round=route.request().url().split('/').at(-2)??''
  writes.push({round,body:route.request().postDataJSON()})
  if(round===at(f.rounds,1).id&&lose){lose=false;const accepted=await route.fetch();expect(accepted.ok()).toBe(true);await route.abort('connectionfailed')}
  else await route.continue()
 })
 await page.goto(f.url);await waitForLive(page)
 for(const name of f.names)await page.getByRole('checkbox',{name,exact:true}).check()
 await page.getByLabel('Kaptein · doble poeng').selectOption(at(f.players,0))
 await page.getByRole('button',{name:'Kopier fireren til andre åpne runder'}).click()
 await page.getByRole('button',{name:'Forbered valgte runder'}).click()
 await page.getByRole('button',{name:'Lagre gjennomgåtte utkast (3)'}).click()
 await expect(page.getByText(/1 av 3 innsendinger bekreftet/)).toContainText('Lagringen er satt på pause')
 expect(writes).toHaveLength(2)
 await page.getByLabel('Runde for Min firer').selectOption(at(f.rounds,1).id)
 await expect(page.getByText(/Lagringen er ikke bekreftet/)).toBeVisible()
 await fantasyLayouts(page,'preset-uncertain')
 await page.getByRole('button',{name:'Avklar samme innsending'}).click()
 await expect(page.getByText('Handlingen er bekreftet. Visningen er oppdatert.')).toBeVisible()
 expect(at(writes,2)).toEqual(at(writes,1))
 await page.getByRole('button',{name:'Lagre gjennomgåtte utkast (1)'}).click()
 await expect(page.getByText(/1 av 1 innsendinger bekreftet/)).toContainText('Alle gjennomgåtte utkast er lagret.')
 expect(writes.map(write=>write.round)).toEqual([at(f.rounds,0).id,at(f.rounds,1).id,at(f.rounds,1).id,at(f.rounds,2).id])
 expect(errors).toEqual([])
 expect(failed).toEqual([`${f.base}/rounds/${at(f.rounds,1).id}/lineup`])
})
test('overview recovers a failed round read and excludes a closed copy destination',async({page})=>{
 await observeLive(page)
 const errors:string[]=[],failed:string[]=[]
 page.on('pageerror',error=>errors.push(error.message))
 page.on('console',message=>{if(message.type()==='error'&&!message.text().includes('Failed to load resource'))errors.push(message.text())})
 page.on('requestfailed',request=>{if(!request.url().includes('/live')&&!request.failure()?.errorText.includes('ERR_ABORTED'))failed.push(new URL(request.url()).pathname)})
 const f=await releaseFixture(page,['individual_stroke_play','individual_stroke_play','individual_stroke_play'],[0,0,0,0])
 const projectionPaths=new Set([`${f.base}/results`,`${f.base}/rounds/${at(f.rounds,0).id}/results`])
 const evidence=observeFantasyReads(page,projectionPaths,`${f.base}/rounds/${at(f.rounds,2).id}`)
 await f.mutate(`${f.base}/rounds/${at(f.rounds,1).id}/deadline`,{deadline:new Date(Date.now()+1500).toISOString()},'PUT')
 let release=()=>{},unavailable=true
 const gate=new Promise<void>(resolve=>{release=resolve})
 await page.route(`**${f.base}/rounds/${at(f.rounds,2).id}`,async route=>{
  if(unavailable){await gate;await route.fulfill({status:503,contentType:'application/json',body:JSON.stringify({error:{code:'unavailable',message:'Injected read failure'}})})}
  else await route.continue()
 })
 await page.goto(f.url);await waitForLive(page)
 const rows=page.locator('.fantasy-presets > li')
 await expect(rows.nth(0)).toContainText('Ikke forberedt')
 await expect(rows.nth(1)).toContainText('Stengt / utilgjengelig')
 await expect(rows.nth(2)).toContainText('Henter runden …')
 await fantasyLayouts(page,'preset-loading')
 release();await expect(rows.nth(2)).toContainText('Kunne ikke hente runden')
 await fantasyLayouts(page,'preset-read-error')
 unavailable=false;await page.getByRole('button',{name:'Oppdater rundeoversikten'}).click()
 await expect(rows.nth(2)).toContainText('Ikke forberedt')
 for(const name of f.names)await page.getByRole('checkbox',{name,exact:true}).check()
 await page.getByLabel('Kaptein · doble poeng').selectOption(at(f.players,0))
 await page.getByRole('button',{name:'Kopier fireren til andre åpne runder'}).click()
 const preview=page.getByRole('region',{name:'Gjennomgå kopiering'})
 await expect(preview.getByText(/Runde 2:/)).toHaveCount(0)
 await expect(preview.getByText(/Runde 3:/)).toBeVisible()
 await page.getByRole('button',{name:'Forbered valgte runder'}).click()
 await expect(rows.nth(1)).not.toContainText('kaptein ×2')
 await expect(page.getByRole('button',{name:'Lagre gjennomgåtte utkast (2)'})).toBeEnabled()
 // Verify recovery of both affected projections, not just the lineup overview.
 await page.getByRole('button',{name:'Poengtavler',exact:true}).click()
 const board=page.locator('#fantasy-boards-panel .fantasy-standings')
 await expect(board).toContainText(at(f.names,0))
 await page.getByLabel('Vis poeng for').selectOption(at(f.rounds,0).id)
 await expect(board).toContainText(at(f.names,0))
 await expect(page.getByText('Oppdaterer runden …')).toHaveCount(0)
 expect((await evidence()).unavailable).toBeGreaterThan(0)
 expect(errors).toEqual([]);expect(failed).toEqual([])
})

test('both projection reads recover a delivered retryable conflict',async({page})=>{
 await observeLive(page)
 const f=await releaseFixture(page,['individual_stroke_play'],[0,0,0,0])
 await page.goto(f.url);await waitForLive(page)
 await page.getByRole('button',{name:'Poengtavler',exact:true}).click()
 const board=page.locator('#fantasy-boards-panel .fantasy-standings')
 await expect(board).toContainText(at(f.names,0))
 await page.getByLabel('Vis poeng for').selectOption(at(f.rounds,0).id)
 await expect(board).toContainText(at(f.names,0))
 await expect(page.getByText('Oppdaterer runden …')).toHaveCount(0)
 const paths=new Set([`${f.base}/results`,`${f.base}/rounds/${at(f.rounds,0).id}/results`])
 const evidence=observeFantasyReads(page,paths,'')
 const injected=new Set<string>()
 for(const path of paths)await page.route(`**${path}`,async route=>{
  if(!injected.has(path)){injected.add(path);await route.fulfill({status:409,contentType:'application/json',body:JSON.stringify({error:{code:'fantasy_conflict',message:'Injected projection conflict'}})})}
  else await route.continue()
 })
 // Both queries are settled and the initial SSE invalidation is over before injection.
 await page.getByRole('button',{name:'Oppdater Fantasy',exact:true}).click()
 await expect.poll(()=>injected.size).toBe(2)
 await expect(page.getByText('Oppdaterer runden …')).toHaveCount(0)
 await expect(page.getByText('Oppdaterer poengtavlene …')).toHaveCount(0)
 await expect(board).toContainText(at(f.names,0))
 await page.getByLabel('Vis poeng for').selectOption('')
 await expect(board).toContainText(at(f.names,0))
 expect([...(await evidence()).conflicts].sort()).toEqual([...paths].sort())
})
