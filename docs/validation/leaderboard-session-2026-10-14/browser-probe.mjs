import { chromium, expect } from '../../../frontend/node_modules/@playwright/test/index.mjs';
const root='http://127.0.0.1:5173';
const trip='00000000-0000-0000-0000-000000002001';
const round='00000000-0000-0000-0000-000000004001';
const draft='00000000-0000-0000-0000-000000004002';
const resultPath=`/api/rounds/${round}/leaderboards/net`;
const url=`/leaderboard?tournament=${trip}&scope=round&round=${round}&metric=net`;
const browser=await chromium.launch({channel:'chrome',headless:true});
const results=[];
try {
 for(const width of [320,390,1280]) {
  const context=await browser.newContext({baseURL:root,viewport:{width,height:900}});
  const page=await context.newPage(); const errors=[];const unexpected=[];let injected=0;
  page.on('pageerror',e=>errors.push(e.message));
  page.on('console',m=>{if(m.type()==='error' && !m.text().startsWith('Failed to load resource:'))errors.push(m.text())});
  page.on('response',r=>{const path=new URL(r.url()).pathname;if(r.status()>=400 && !(path===resultPath && r.status()===503) && !(path==='/api/auth/session' && r.status()===401))unexpected.push(`${r.status()} ${path}`)});
  page.on('requestfailed',r=>{if(!r.failure()?.errorText.includes('ERR_ABORTED'))unexpected.push(`${r.failure()?.errorText} ${new URL(r.url()).pathname}`)});
  await page.goto('/login');await page.getByLabel('Brukernavn',{exact:true}).fill('admin');
  await page.getByLabel('Passord',{exact:true}).fill('golf-dev-2026');
  await page.getByRole('button',{name:'Logg inn',exact:true}).click();await expect(page).not.toHaveURL(/\/login/);
  const session=await(await page.request.get('/api/auth/session')).json();
  let release;let seen;let mode='delay';
  const gate=new Promise(r=>release=r);const started=new Promise(r=>seen=r);
  await page.route('**'+resultPath,async route=>{
   if(mode==='delay'){seen();await gate;return route.continue()}
   if(mode==='error'){injected++;return route.fulfill({status:503,headers:{'cache-control':'private, no-store'},json:{error:{code:'service_unavailable',message:'service is not ready'}}})}
   return route.continue();
  });
  await page.goto(url);await started;
  await expect(page.getByRole('status').filter({hasText:'Laster …'}).first()).toBeVisible();
  mode='pass';release();
  await expect(page.locator('.leaderboard-row')).toHaveCount(4);
  expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
  await page.screenshot({path:`/tmp/golf-a1-populated-${width}.png`,fullPage:true});
  const endpoints=[];
  for(const base of [`/api/rounds/${round}`,`/api/tournaments/${trip}`])for(const metric of ['gross','net']){
   const r=await page.request.get(`${base}/leaderboards/${metric}`);expect(r.status()).toBe(200);
   expect(r.headers()['cache-control']).toBe('private, no-store');const b=await r.json();expect(b.entries.length).toBeGreaterThan(0);endpoints.push({scope:base.includes('/rounds/')?'round':'tournament',metric,status:r.status(),entries:b.entries.length});
  }
  mode='error';await page.reload();await expect(page.getByRole('alert')).toBeVisible();
  await expect(page.locator('.leaderboard-row')).toHaveCount(0);
  await page.screenshot({path:`/tmp/golf-a1-error-${width}.png`,fullPage:true});
  mode='pass';await page.getByRole('button',{name:'Prøv igjen',exact:true}).click();
  await expect(page.locator('.leaderboard-row')).toHaveCount(4);
  await page.goto(`/leaderboard?tournament=${trip}&scope=round&round=${draft}&metric=net`);
  await expect(page.getByText('Runden er fortsatt en kladd. Resultater vises når runden åpnes.')).toBeVisible();
  expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
  await page.screenshot({path:`/tmp/golf-a1-empty-${width}.png`,fullPage:true});
  const logout=await page.request.post('/api/auth/logout',{headers:{'x-csrf-token':session.csrf_token}});expect(logout.status()).toBe(204);
  for(const base of [`/api/rounds/${round}`,`/api/tournaments/${trip}`])for(const metric of ['gross','net']){
   const r=await page.request.get(`${base}/leaderboards/${metric}`);expect(r.status()).toBe(401);expect(r.headers()['cache-control']).toBe('private, no-store');expect((await r.json()).error.code).toBe('unauthenticated');
  }
  await page.reload();await expect(page).toHaveURL(/\/login/);
  expect(errors).toEqual([]);expect(unexpected).toEqual([]);
  results.push({width,loading:true,populated:true,long_content_no_overflow:true,error_and_read_retry:true,empty:true,unauthenticated_all_four_routes:true,injected_503_responses:injected,page_errors:errors,unexpected_network_failures:unexpected,endpoints});
  await context.close();
 }
 console.log(JSON.stringify(results,null,2));
}finally{await browser.close()}
