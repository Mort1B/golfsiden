import { expect, test, type Page } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { fantasyFixture, fantasyLayouts } from './fantasySupport'

test.skip(process.env.GOLF_RELIABILITY_BROWSER!=='1','Requires a disposable local API and Chrome.')
test.use({screenshot:'off',trace:'off'})
function observe(page:Page){
 const errors:string[]=[],bad:string[]=[]
 page.on('pageerror',()=>errors.push('pageerror'))
 page.on('console',m=>{if(m.type()==='error'&&!m.text().startsWith('Failed to load resource:'))errors.push('console error')})
 page.on('response',r=>{const path=new URL(r.url()).pathname;if(r.status()>=400&&!(r.status()===409&&path.includes('/fantasy/'))&&!(r.status()===503&&path.includes('/fantasy/'))&&!(r.status()===401&&path==='/api/auth/session'))bad.push(`${r.status()} ${path}`)})
 page.on('requestfailed',r=>{if(!r.url().endsWith('/live')&&!['net::ERR_ABORTED','net::ERR_FAILED'].includes(r.failure()?.errorText??''))bad.push(new URL(r.url()).pathname)})
 return ()=>{expect(errors).toEqual([]);expect(bad).toEqual([])}
}
async function setup(page:Page){const f=await fantasyFixture(page);await f.mutate(`/api/tournaments/${f.tournament.id}/fantasy`,{enabled:true},'PUT');await f.mutate(`/api/tournaments/${f.tournament.id}/fantasy/entry`);return f}
async function choose(page:Page,f:Awaited<ReturnType<typeof setup>>){for(const name of f.names)await page.getByRole('checkbox',{name,exact:true}).check();await page.getByLabel('Kaptein · doble poeng').selectOption(f.players[0]??'')}

test('competing session supersedes an unaccepted request; exact reconciliation then deliberate rebase',async({page,browser})=>{
 const clean=observe(page),f=await setup(page),round=f.rounds[0];if(!round)throw new Error('round')
 const context=await browser.newContext({baseURL:'http://127.0.0.1:5173'})
 try{
  const login=await context.request.post('/api/auth/login',{data:{username:f.username,password:f.password}});expect(login.status()).toBe(200)
  const second=decodeAuthSession(await login.json())
  await page.goto(f.url);await choose(page,f)
  const bodies:{request_id:string;expected_revision:number;picks:string[];captain:string}[]=[]
  const endpoint=`/api/tournaments/${f.tournament.id}/fantasy/rounds/${round.id}/lineup`
  await page.route(`**${endpoint}`,async route=>{bodies.push(route.request().postDataJSON());if(bodies.length===1)return route.abort('failed');return route.continue()})
  await page.getByRole('button',{name:'Lagre firer og kaptein'}).click();await expect(page.getByText(/Lagringen er ikke bekreftet/)).toBeVisible()
  const original=bodies[0];if(!original)throw new Error('request')
  const competing=await context.request.put(endpoint,{headers:{'x-csrf-token':second.csrf_token},data:{...original,request_id:crypto.randomUUID(),captain:f.players[1]}});expect(competing.status()).toBe(200)
  await page.getByRole('button',{name:'Avklar samme innsending'}).click()
  await expect(page.getByText(/Den opprinnelige innsendingen ble ikke lagret/)).toBeVisible()
  expect(bodies[1]).toEqual(original)
  await fantasyLayouts(page,'followup-superseded')
  await page.getByRole('button',{name:'Behold valgene som nytt utkast'}).click();expect(bodies).toHaveLength(2)
  await expect(page.getByLabel('Kaptein · doble poeng')).toHaveValue(original.captain)
  await page.getByRole('button',{name:'Lagre firer og kaptein'}).click();await expect(page.getByText('Lagret lag · revisjon 2')).toBeVisible()
  expect(bodies[2]?.request_id).not.toBe(original.request_id);expect(bodies[2]?.expected_revision).toBe(1);expect(bodies[2]?.picks).toEqual(original.picks);expect(bodies[2]?.captain).toBe(original.captain)
  clean()
 }finally{await context.close()}
})

for(const exit of ['Score','logout'])test(`acknowledged read failure permits ${exit}`,async({page})=>{
 const clean=observe(page),f=await setup(page),round=f.rounds[0];if(!round)throw new Error('round')
 await page.goto(f.url);await choose(page,f)
 let writes=0
 page.on('request',r=>{if(r.method()==='PUT'&&r.url().endsWith('/lineup'))writes++})
 await page.route(`**/api/tournaments/${f.tournament.id}/fantasy/rounds/${round.id}`,route=>route.fulfill({status:503,json:{error:{code:'unavailable',message:'Injected refresh failure'}}}))
 await page.getByRole('button',{name:'Lagre firer og kaptein'}).click();await expect(page.getByText(/Handlingen er bekreftet, men/)).toBeVisible()
 await expect(page.getByText(/Kvittering beholdt/)).toBeVisible()
 if(exit==='Score'){await page.getByRole('link',{name:'Score',exact:true}).click();await expect(page).toHaveURL(/\/score/)}
 else {await page.getByRole('button',{name:'Logg ut',exact:true}).click();await expect(page.getByRole('button',{name:'Logg ut',exact:true})).toHaveCount(0);expect((await page.request.get('/api/auth/session')).status()).toBe(401)}
 expect(writes).toBe(1);clean()
})

test('inline manager and golfer details retain selected round after private query clearing',async({page})=>{
 const clean=observe(page),f=await setup(page),round=f.rounds[1];if(!round)throw new Error('round')
 await page.addInitScript(()=>{const sources:EventSource[]=[],Native=window.EventSource;window.EventSource=class extends Native{constructor(url:string|URL,init?:EventSourceInit){super(url,init);sources.push(this)}};Object.defineProperty(window,'followupSignal',{value:(name:string)=>{for(const source of sources){source.close();source.dispatchEvent(new Event(name))}}})})
 const signal=async(name:string)=>page.evaluate(value=>{if('followupSignal'in window&&typeof window.followupSignal==='function')window.followupSignal(value)},name)
 await page.goto(f.url)
 await page.getByLabel('Runde for Min firer').selectOption(round.id)
 await page.getByRole('button',{name:'Poengtavler',exact:true}).click()
 await page.getByLabel('Vis poeng for').selectOption(round.id)
 for(const kind of ['Fantasy-lag','Spillerpoeng']){
  await page.getByRole('button',{name:kind,exact:true}).click()
  const row=page.locator('.fantasy-standings > li').last(),button=row.locator(':scope > button')
  await button.click();await expect(button).toHaveAttribute('aria-expanded','true');await expect(row.getByRole('heading')).toBeVisible()
  await expect(row.locator('details')).toHaveCount(1);await expect(row.locator('summary')).toContainText('Runde 2:')
  await fantasyLayouts(page,`followup-inline-${kind==='Fantasy-lag'?'manager':'golfer'}`)
  await signal('error');await expect(page.locator('.fantasy-detail')).toHaveCount(0)
  await signal('open');await expect(page.getByRole('button',{name:kind,exact:true})).toHaveAttribute('aria-pressed','true')
  await expect(page.getByLabel('Runde for Min firer')).toHaveValue(round.id);await expect(page.getByLabel('Vis poeng for')).toHaveValue(round.id)
  await expect(row.getByRole('heading')).toBeVisible();await expect(row.locator('details')).toHaveCount(1)
  await row.getByRole('button',{name:'Lukk poengdetaljer'}).focus();await page.keyboard.press('Escape');await expect(button).toBeFocused();await expect(button).toHaveAttribute('aria-expanded','false')
 }
 clean()
})
