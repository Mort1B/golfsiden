import { test, expect } from '/home/morten/Prog/guttasgolfside/golfsiden/frontend/node_modules/@playwright/test/index.mjs'
import { matchFixture } from '/home/morten/Prog/guttasgolfside/golfsiden/frontend/e2e/matchSupport'
import { offlineFixture, queueRows } from '/home/morten/Prog/guttasgolfside/golfsiden/frontend/e2e/offlineSupport'
for (const failedSave of [false, true]) test(`PERSIST-1 same-account rotation loses ${failedSave ? 'failed-save' : 'unsaved'} match note`, async ({page,browser}) => {
 await page.setViewportSize({width:390,height:844})
 const f=await matchFixture(page,browser)
 await page.goto(f.url)
 const input=page.getByLabel(`Notat · ${f.firstName}`,{exact:true})
 await input.fill('7')
 if(failedSave) {
  await page.evaluate(()=>{const put=IDBObjectStore.prototype.put;IDBObjectStore.prototype.put=function(value,key){if(this.name==='matches')throw new DOMException('Synthetic failure','QuotaExceededError');return key===undefined?put.call(this,value):put.call(this,value,key)}})
  await page.getByRole('button',{name:'Lagre notat',exact:true}).first().click()
  await expect(page.getByRole('alert')).toBeVisible()
 }
 await expect(page.getByRole('button',{name:'Logg ut',exact:true})).toBeDisabled()
 // Control: page return without rotation preserves input and unsafe guard.
 const control=page.waitForResponse(r=>r.url().endsWith('/api/auth/session')&&r.status()===200)
 await page.evaluate(()=>window.dispatchEvent(new PageTransitionEvent('pageshow',{persisted:true})))
 await control
 await expect(input).toHaveValue('7')
 await expect(page.getByRole('button',{name:'Logg ut',exact:true})).toBeDisabled()
 await page.screenshot({path:`/tmp/golf-persistence-assessment/match-before-${failedSave}.png`,fullPage:true})
 // A second login in the same browser cookie jar gives the same user a new session.
 const login=await page.request.post('/api/auth/login',{data:{username:f.auth.username,password:f.password}})
 expect(login.status()).toBe(200)
 const session=await login.json()
 expect(session.user_id===f.auth.user_id).toBe(true)
 expect(session.csrf_token!==f.auth.csrf_token).toBe(true)
 const refreshed=page.waitForResponse(r=>r.url().endsWith('/api/auth/session')&&r.status()===200)
 await page.evaluate(()=>window.dispatchEvent(new PageTransitionEvent('pageshow',{persisted:true})))
 await refreshed
 await expect(input).toHaveValue('')
 await expect(page.getByRole('button',{name:'Forkast ulagrede notater',exact:true})).toHaveCount(0)
 await expect(page.getByRole('button',{name:'Logg ut',exact:true})).toBeEnabled()
 expect((await f.read()).notes.length).toBe(0)
 await page.screenshot({path:`/tmp/golf-persistence-assessment/match-after-${failedSave}.png`,fullPage:true})
 console.log(`PERSIST-1 failedSave=${failedSave}: same-user return preserved 7; new same-user session removed note and guard; server notes=0`)
})
test('PERSIST-2 bounded storage outage repeatedly drains before a timer can run', async ({page,context})=>{
 const f=await offlineFixture(page)
 await page.goto(f.url())
 await expect(page.getByRole('button',{name:/Registrer par/})).toBeEnabled()
 await context.setOffline(true)
 await page.getByRole('button',{name:/Registrer par/}).click()
 await expect.poll(async()=>(await queueRows(page)).length).toBe(1)
 const observed=await page.evaluate(async()=>{
  const original=indexedDB
  let attempts=0, recovered=false
  Object.defineProperty(window,'indexedDB',{configurable:true,get(){attempts++;if(attempts<=50)throw new Error('Synthetic bounded storage outage');recovered=true;return original}})
  Object.defineProperty(navigator,'onLine',{configurable:true,get:()=>true})
  const timer=new Promise<{attempts:number;recovered:boolean}>(resolve=>setTimeout(()=>resolve({attempts,recovered}),0))
  window.dispatchEvent(new Event('online'))
  const result=await timer
  Object.defineProperty(window,'indexedDB',{configurable:true,value:original})
  delete (navigator as {onLine?:boolean}).onLine
  return result
 })
 expect(observed.attempts).toBeGreaterThanOrEqual(50)
 expect(observed.recovered).toBe(true)
 expect(await queueRows(page)).toHaveLength(1)
 console.log(`PERSIST-2 storageAttemptsBeforeFirstTimer=${observed.attempts}; test restores storage after 50 failures; queued edit retained`)
})
test('PERSIST-1 durable match note survives same-account session replacement',async({page,browser,context})=>{
 const f=await matchFixture(page,browser)
 await page.goto(f.url)
 await expect(page.getByLabel(`Notat · ${f.firstName}`,{exact:true})).toBeVisible()
 await context.setOffline(true)
 await page.getByLabel(`Notat · ${f.firstName}`,{exact:true}).fill('7')
 await page.getByRole('button',{name:'Lagre notat',exact:true}).first().click()
 await expect(page.getByRole('region',{name:'Lokale matchendringer'})).toContainText('7 slag på hull 1')
 await page.route('**/commands',route=>route.fulfill({status:503,json:{error:{code:'unavailable',message:'synthetic temporary outage'}}}))
 await context.setOffline(false)
 const login=await page.request.post('/api/auth/login',{data:{username:f.auth.username,password:f.password}})
 expect(login.status()).toBe(200)
 const session=await login.json()
 expect(session.user_id===f.auth.user_id&&session.csrf_token!==f.auth.csrf_token).toBe(true)
 const refresh=page.waitForResponse(r=>r.url().endsWith('/api/auth/session')&&r.status()===200)
 await page.evaluate(()=>window.dispatchEvent(new PageTransitionEvent('pageshow',{persisted:true})))
 await refresh
 await expect(page.getByRole('region',{name:'Lokale matchendringer'})).toContainText('7 slag på hull 1')
 await page.unroute('**/commands')
 await expect.poll(async()=>(await f.read()).notes[0]?.gross_strokes,{timeout:30000}).toBe(7)
 const storage=await page.evaluate(async()=>({local:localStorage.length,session:sessionStorage.length,caches:await caches.keys(),workers:(await navigator.serviceWorker.getRegistrations()).length}))
 expect(storage).toEqual({local:0,session:0,caches:[],workers:0})
 console.log('PERSIST-1 durable control: queued note survives rotation and server receives 7; no localStorage/sessionStorage/CacheStorage/service-worker entries')
})
