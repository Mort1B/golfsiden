const {chromium,expect}=require(process.cwd()+'/frontend/node_modules/@playwright/test');
const fs=require('node:fs'); const assert=require('node:assert/strict');
const f=JSON.parse(fs.readFileSync('/tmp/golf-rehearsal-738ec1d/fixture-private.json'));const origin='https://localhost:18592';
(async()=>{
const browser=await chromium.launch({channel:'chrome'});console.log('Browser '+browser.version());
try {
 for(const [role,user,scores] of [['admin',f.adminName,['74 brutto','92 brutto']],['member',f.memberName,['36 brutto','45 brutto']]]) {
  const context=await browser.newContext({baseURL:origin,ignoreHTTPSErrors:true});const page=await context.newPage();const errors=[];
  page.on('pageerror',e=>errors.push(e.name));page.on('console',m=>{if(m.type()==='error'&&!m.text().startsWith('Failed to load resource:'))errors.push('console error')});
  const bad=[];page.on('response',r=>{if(r.status()>=400&&!(r.status()===401&&new URL(r.url()).pathname==='/api/auth/session'))bad.push(r.status())});
  await page.goto('/login');await page.getByLabel('Brukernavn',{exact:true}).fill(user);await page.getByLabel('Passord',{exact:true}).fill(f.password);await page.getByRole('button',{name:'Logg inn',exact:true}).click();await expect(page).not.toHaveURL(/\/login/);
  for(const [width,height] of [[390,844],[1280,900]]) {
   await page.setViewportSize({width,height});await page.goto(`/leaderboard?tournament=${f.tournament.id}&round=${f.round.id}&scope=round&metric=gross`);
   await expect(page.locator('.leaderboard-score span')).toHaveText(scores);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
   if(width===390) { await page.locator('.leaderboard-row-link').last().focus(); await expect.poll(async()=>page.locator('.leaderboard-row-link').last().evaluate(e=>e.getBoundingClientRect().bottom<=document.querySelector('.bottom-nav').getBoundingClientRect().top)).toBe(true); }
   await page.screenshot({path:`docs/validation/recovery-2026-09-23/restored-${role}-${width}.png`,fullPage:true});
  }
  assert.deepEqual(errors,[]);assert.deepEqual(bad,[]);console.log(`Restored ${role}: UI login, exact persisted totals at 390x844 and 1280x900, no overflow or unexpected console/HTTP errors.`);await context.close();
 }
}finally{await browser.close()}
})().catch(e=>{console.error(e.message);process.exitCode=1});
