import { expect, test, request, type Page } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeObject } from '../src/api/decoder'
import { decodeClaimReceipt } from '../src/api/playerClaimDecoders'
import { fantasyFixture, fantasyLayouts } from './fantasySupport'

test.skip(process.env.GOLF_RELIABILITY_BROWSER!=='1','Requires a disposable local API and Chrome.')
test.use({screenshot:'off',trace:'off'})
function observe(page:Page){
  const errors:string[]=[],responses:{path:string;status:number}[]=[],failures:{path:string;error:string|undefined}[]=[]
  page.on('pageerror',()=>errors.push('pageerror'))
  page.on('console',message=>{if(message.type()==='error'&&!message.text().startsWith('Failed to load resource:'))errors.push('console error')})
  page.on('response',response=>{if(response.status()>=400)responses.push({path:new URL(response.url()).pathname,status:response.status()})})
  page.on('requestfailed',req=>failures.push({path:new URL(req.url()).pathname,error:req.failure()?.errorText}))
  return {errors,responses,failures}
}
async function controlledLive(page:Page){
  await page.addInitScript(()=>{
    const sources:EventSource[]=[]
    const Native=window.EventSource
    window.EventSource=class extends Native {constructor(url:string|URL,init?:EventSourceInit){super(url,init);sources.push(this)}}
    Object.defineProperty(window,'reliabilitySignal',{value:(signal:string)=>{for(const source of sources){source.close();source.dispatchEvent(new Event(signal))}}})
  })
}
async function signal(page:Page,name:string){await page.evaluate(value=>{if('reliabilitySignal' in window&&typeof window.reliabilitySignal==='function')window.reliabilitySignal(value)},name)}

test('Fantasy retains drafts, exact uncertain requests and acknowledged receipts across parent clearing',async({page})=>{
  const events=observe(page),f=await fantasyFixture(page),round=f.rounds[0],second=f.rounds[1]
  if(!round||!second||!f.players[0]||!f.players[1])throw new Error('Missing fixture')
  await f.mutate(`/api/tournaments/${f.tournament.id}/fantasy`,{enabled:true},'PUT')
  await f.mutate(`/api/tournaments/${f.tournament.id}/fantasy/entry`)
  await controlledLive(page)
  await page.goto(f.url)
  for(const name of f.names)await page.getByRole('checkbox',{name,exact:true}).check()
  await page.getByLabel('Kaptein · doble poeng').selectOption(f.players[0])
  await signal(page,'error')
  await expect(page.getByLabel('Kaptein · doble poeng')).toHaveCount(0)
  await expect(page.getByText(/Fantasy-valgene beholdes/)).toBeVisible()
  await fantasyLayouts(page,'reliability-cleared')
  await signal(page,'open')
  await expect(page.getByLabel('Kaptein · doble poeng')).toHaveValue(f.players[0])
  await expect(page.locator('.fantasy-picks input:checked')).toHaveCount(4)
  await page.getByLabel('Runde for Min firer').selectOption(second.id)
  await expect(page.locator('.fantasy-picks input:checked')).toHaveCount(0)
  await page.getByLabel('Runde for Min firer').selectOption(round.id)
  await expect(page.getByLabel('Kaptein · doble poeng')).toHaveValue(f.players[0])
  const bodies:unknown[]=[]
  const path=`**/api/tournaments/${f.tournament.id}/fantasy/rounds/${round.id}/lineup`
  await page.route(path,async route=>{
    bodies.push(route.request().postDataJSON())
    const response=await route.fetch()
    expect(response.ok()).toBe(true)
    if(bodies.length===1)return route.abort('failed')
    return route.fulfill({response})
  })
  await page.getByRole('button',{name:'Lagre firer og kaptein'}).click()
  await expect(page.getByText(/Lagringen er ikke bekreftet/)).toBeVisible()
  await signal(page,'error');await expect(page.getByLabel('Kaptein · doble poeng')).toHaveCount(0)
  await signal(page,'open');await expect(page.getByText(/Lagringen er ikke bekreftet/)).toBeVisible()
  await fantasyLayouts(page,'reliability-uncertain')
  await page.getByRole('button',{name:'Avklar samme innsending'}).click()
  await expect(page.getByText('Handlingen er bekreftet. Visningen er oppdatert.')).toBeVisible()
  expect(bodies).toHaveLength(2);expect(bodies[1]).toEqual(bodies[0])
  await page.getByLabel('Kaptein · doble poeng').selectOption(f.players[1])
  let failRead=true
  await page.route(`**/api/tournaments/${f.tournament.id}/fantasy/rounds/${round.id}`,async route=>failRead?route.fulfill({status:503,json:{error:{code:'unavailable',message:'Injected read failure'}}}):route.continue())
  await page.getByRole('button',{name:'Lagre firer og kaptein'}).click()
  await expect(page.getByText(/Handlingen er bekreftet, men visningen kunne ikke oppdateres/)).toBeVisible()
  await expect(page.getByText(/Kvittering beholdt: innsending revisjon 2/)).toBeVisible()
  await expect(page.getByText(/Lagringen er ikke bekreftet/)).toHaveCount(0)
  await fantasyLayouts(page,'reliability-refresh-failed')
  failRead=false
  await page.getByRole('button',{name:'Oppdater Fantasy'}).click()
  await expect(page.getByText('Handlingen er bekreftet. Visningen er oppdatert.')).toBeVisible()
  await expect(page.getByText('Lagret lag · revisjon 2')).toBeVisible()
  expect(bodies).toHaveLength(3)
  await fantasyLayouts(page,'reliability-recovered')
  expect(events.errors).toEqual([])
  expect(events.responses.every(item=>item.status===503&&item.path.endsWith(`/fantasy/rounds/${round.id}`)||item.status===409&&item.path.endsWith('/results'))).toBe(true)
  expect(events.failures.every(item=>item.path.endsWith('/live')||item.path.endsWith('/lineup')&&item.error==='net::ERR_FAILED'||item.error==='net::ERR_ABORTED')).toBe(true)
})

for(const cookie of [true,false])test(`committed player claim with lost response, cookie=${cookie}`,async({page,browser})=>{
  const f=await fantasyFixture(page),prepared=f.players[1]
  if(!prepared)throw new Error('Missing prepared player')
  const claim=decodeClaimReceipt(await(await f.mutate(`/api/tournaments/${f.tournament.id}/players/${prepared}/claim`)).json())
  const context=await browser.newContext({baseURL:'http://127.0.0.1:5173'}),committer=await request.newContext({baseURL:'http://127.0.0.1:5173'})
  try{
    const recipient=await context.newPage(),events=observe(recipient),username=`recover_${cookie?'c':'n'}_${Date.now()}`,password='reliability claim password'
    let registrations=0
    await recipient.route(`**/api/player-claims/${claim.claim_id}/register`,async route=>{
      registrations++
      const response=await committer.post(`/api/player-claims/${claim.claim_id}/register`,{data:route.request().postDataJSON()})
      expect(response.status()).toBe(201)
      const committed=decodeObject(await response.json(),'claim')
      expect(decodeAuthSession(committed.session).player_id).toBe(prepared)
      const headers:Record<string,string>={'content-type':'application/json'}
      if(cookie){const value=response.headers()['set-cookie'];if(!value)throw new Error('Missing cookie');headers['set-cookie']=value}
      await route.fulfill({status:201,headers,body:'{'})
    })
    await recipient.goto(`/claim/${claim.claim_id}#token=${claim.token}`)
    await recipient.getByLabel('Brukernavn',{exact:true}).fill(username)
    await recipient.getByLabel('Passord',{exact:true}).fill(password)
    await recipient.getByRole('button',{name:'Ta i bruk kontoen'}).click()
    if(cookie)await expect(recipient.getByText(/Kontoen er klar/)).toBeVisible()
    else {
      await expect(recipient.getByText(/Kontoen kan allerede være opprettet/)).toBeVisible()
      expect((await recipient.request.get('/api/auth/session')).status()).toBe(401)
    }
    for(const width of [320,390,1280]){
      await recipient.setViewportSize({width,height:900})
      expect(await recipient.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
      for(const link of await recipient.locator('.claim-page a:visible').all())expect((await link.boundingBox())?.height).toBeGreaterThanOrEqual(44)
      await recipient.screenshot({path:`/tmp/golf-reliability-claim-${cookie}-${width}.png`,fullPage:true})
    }
    if(!cookie){
      await recipient.getByRole('button',{name:'Kontroller kontolenken på nytt'}).click()
      await expect(recipient.getByText(/Kontolenken er ikke tilgjengelig/)).toBeVisible()
      await expect(recipient.getByRole('button',{name:'Ta i bruk kontoen'})).toHaveCount(0)
      await recipient.getByRole('link',{name:`Logg inn med ${username}`}).click()
      await expect(recipient.getByLabel('Brukernavn',{exact:true})).toHaveValue(username)
      await recipient.getByLabel('Passord',{exact:true}).fill(password)
      await recipient.getByRole('button',{name:'Logg inn',exact:true}).click()
      await expect(recipient.getByRole('button',{name:'Logg ut'})).toBeVisible()
    }
    expect(decodeAuthSession(await(await recipient.request.get('/api/auth/session')).json()).player_id).toBe(prepared)
    expect((await committer.post(`/api/player-claims/${claim.claim_id}/preview`,{data:{token:claim.token}})).status()).toBe(410)
    expect(registrations).toBe(1)
    expect(events.errors).toEqual([])
    expect(events.responses.every(item=>item.path==='/api/auth/session'&&item.status===401||item.path.endsWith('/preview')&&item.status===410)).toBe(true)
    expect(events.failures.every(item=>item.error==='net::ERR_ABORTED')).toBe(true)
  }finally{await context.close();await committer.dispose()}
})


test('precommit registration failure revalidates an available claim before deliberate resubmission',async({page,browser})=>{
 const f=await fantasyFixture(page),prepared=f.players[1];if(!prepared)throw new Error('player')
 const claim=decodeClaimReceipt(await(await f.mutate(`/api/tournaments/${f.tournament.id}/players/${prepared}/claim`)).json())
 const context=await browser.newContext({baseURL:'http://127.0.0.1:5173'})
 try{
  const recipient=await context.newPage(),events=observe(recipient),username=`precommit_${Date.now()}`
  let registrations=0
  await recipient.route(`**/api/player-claims/${claim.claim_id}/register`,async route=>{registrations++;if(registrations===1)return route.fulfill({status:503,json:{error:{code:'unavailable',message:'Injected before commit'}}});return route.continue()})
  await recipient.goto(`/claim/${claim.claim_id}#token=${claim.token}`)
  await recipient.getByLabel('Brukernavn',{exact:true}).fill(username);await recipient.getByLabel('Passord',{exact:true}).fill('reliability claim password');await recipient.getByRole('button',{name:'Ta i bruk kontoen'}).click()
  await expect(recipient.getByText(/Kontoen kan allerede være opprettet/)).toBeVisible()
  await recipient.getByRole('button',{name:'Kontroller kontolenken på nytt'}).click()
  await expect(recipient.getByText(/Kontolenken er fortsatt tilgjengelig/)).toBeVisible();expect(registrations).toBe(1)
  await expect(recipient.getByLabel('Brukernavn',{exact:true})).toHaveValue(username);await expect(recipient.getByLabel('Passord',{exact:true})).toHaveValue('')
  expect(new URL(recipient.url()).hash).toBe('')
  for(const width of [320,390,1280]){await recipient.setViewportSize({width,height:900});expect(await recipient.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);await recipient.screenshot({path:`/tmp/golf-followup-claim-available-${width}.png`,fullPage:true})}
  await recipient.getByLabel('Passord',{exact:true}).fill('reliability claim password');await recipient.getByRole('button',{name:'Ta i bruk kontoen'}).click()
  await expect(recipient.getByText(/Kontoen er klar/)).toBeVisible();expect(registrations).toBe(2)
  expect(decodeAuthSession(await(await recipient.request.get('/api/auth/session')).json()).player_id).toBe(prepared)
  expect(events.errors).toEqual([]);expect(events.responses.every(item=>item.path==='/api/auth/session'&&item.status===401||item.path.endsWith('/register')&&item.status===503)).toBe(true);expect(events.failures.every(item=>item.error==='net::ERR_ABORTED')).toBe(true)
 }finally{await context.close()}
})
