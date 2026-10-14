import {chromium, expect, request} from '../../../frontend/node_modules/@playwright/test/index.mjs';

const root='http://127.0.0.1:5173', trip='00000000-0000-0000-0000-000000002001';
const round='00000000-0000-0000-0000-000000004001', team='00000000-0000-0000-0000-000000005001';
const cardPath=`/api/rounds/${round}/scorecards/team/${team}`;
const accessPath=`/api/rounds/${round}/score-access`, scoringPath=cardPath+'/scoring';
const scoreUrl=`/score?tournament=${trip}&round=${round}&owner_type=team&owner=${team}&view=summary`;
const memberUrl=`/tournaments/${trip}/rounds/${round}/scorecards/team/${team}?metric=gross&view=summary`;
const longName='Syntetisk lag med et langt navn for kontroll av sikre scorekort på smal mobil og stor skjerm';
const out='/tmp/golf-a1g-';
const browser=await chromium.launch({channel:'chrome',headless:true}),results=[];
try {
  // Prepare synthetic facts through existing mutation APIs before read observers.
  const fixture=await request.newContext({baseURL:root});
  const login=await fixture.post('/api/auth/login',{data:{username:'admin',password:'golf-dev-2026'}});expect(login.status()).toBe(200);
  const auth=await login.json(), headers={'x-csrf-token':auth.csrf_token};
  const tournament=await(await fixture.get(`/api/tournaments/${trip}`)).json();
  if(tournament.status==='draft')expect((await fixture.post(`/api/tournaments/${trip}/start`,{headers,data:{expected_tournament_updated_at:tournament.updated_at}})).status()).toBe(200);
  const pairings=await(await fixture.get(`/api/rounds/${round}/pairings`)).json();
  if(pairings.status==='draft') {
    const changed=await fixture.put(`/api/rounds/${round}/pairings`,{headers,data:{expected_round_updated_at:pairings.updated_at,
      teams:pairings.teams.map(t=>({id:t.id,name:t.id===team?longName:t.name,members:t.members.map(m=>({player_id:m.player_id})),schedule_flight_id:null})),
      flights:pairings.flights.map(f=>({id:f.id,name:f.name,starting_hole:f.starting_hole,tee_time:f.tee_time,members:f.members.map(m=>({player_id:m.player_id}))})),legacy_conversions:[]}});
    expect(changed.status()).toBe(200);expect((await fixture.post(`/api/rounds/${round}/open`,{headers,data:{}})).status()).toBe(200);
  } else {expect(pairings.status).toBe('open');expect(pairings.teams.find(t=>t.id===team).name).toBe(longName);}
  const initial=await(await fixture.get(scoringPath)).json();
  if(initial.holes_scored===0)expect((await fixture.put(`/api/rounds/${round}/scores`,{headers,data:{hole_id:initial.holes[0].hole_id,owner:{type:'team',id:team},gross_strokes:5}})).status()).toBe(200);
  expect((await fixture.post('/api/auth/logout',{headers})).status()).toBe(204);await fixture.dispose();
  for(const width of [320,390,1280]) {
    const context=await browser.newContext({baseURL:root,viewport:{width,height:900}}),page=await context.newPage();
    const errors=[],unexpected=[],mutations=[],expectedFailures=new Set();
    page.on('pageerror',e=>errors.push(e.message));
    page.on('console',m=>{if(m.type()==='error'&&!m.text().startsWith('Failed to load resource:'))errors.push(m.text());});
    page.on('response',r=>{const p=new URL(r.url()).pathname;if(r.status()>=400&&!expectedFailures.has(`${r.status()} ${p}`)&&!(p==='/api/auth/session'&&r.status()===401))unexpected.push(`${r.status()} ${p}`);});
    page.on('requestfailed',r=>{if(!r.failure()?.errorText.includes('ERR_ABORTED')&&!r.url().includes('/live'))unexpected.push(new URL(r.url()).pathname);});
    page.on('request',r=>{const p=new URL(r.url()).pathname;if(r.method()!=='GET'&&p.startsWith('/api/rounds/'))mutations.push(`${r.method()} ${p}`);});
    await page.goto('/login?returnTo=%2Ftournaments');await page.getByLabel('Brukernavn',{exact:true}).fill('admin');await page.getByLabel('Passord',{exact:true}).fill('golf-dev-2026');await page.getByRole('button',{name:'Logg inn',exact:true}).click();await expect(page).toHaveURL(/\/tournaments$/);
    const session=await(await page.request.get('/api/auth/session')).json();
    const decoded=await page.evaluate(async({round,team})=>{
      const {api}=await import('/src/api/client.ts'),owner={type:'team',id:team};
      const [access,member,scoring]=await Promise.all([api.scoreAccess(round),api.scorecardRead(round,owner),api.scorecardScoring(round,owner)]);
      return {owners:access.writable_owners.length,member:member.projection,scoring:scoring.projection,holes:member.holes.length,scored:scoring.holes_scored};
    },{round,team});
    expect(decoded).toEqual({owners:4,member:'read',scoring:'scoring',holes:18,scored:1});
    for(const [kind,path,url] of [['access',accessPath,scoreUrl],['member',cardPath,memberUrl],['scoring',scoringPath,scoreUrl]]) {
      console.error(`${width}px ${kind} read`);
      const actual=await page.request.get(path);expect(actual.status()).toBe(200);expect(actual.headers()['cache-control']).toBe('private, no-store');const model=await actual.json();
      let release,started;const gate=new Promise(r=>release=r),seen=new Promise(r=>started=r);let mode='delay',injected=0;
      await page.route('**'+path,async route=>{
        if(mode==='delay'){started();await gate;return route.continue();}
        if(mode==='error'||mode==='denied'){const status=mode==='error'?503:403;expectedFailures.add(`${status} ${path}`);injected++;return route.fulfill({status,headers:{'cache-control':'private, no-store'},json:{error:{code:status===503?'service_unavailable':'forbidden',message:'score read unavailable'}}});}
        if(mode==='empty')return route.fulfill({headers:{'cache-control':'private, no-store'},json:kind==='access'?{...model,writable_owners:[]}:{...model,holes:model.holes.map(h=>({...h,score:null,net_strokes:null})),holes_scored:0,gross_total:0,net_total:0,complete:false,confirmed:false,confirmed_at:null,...(kind==='scoring'?{confirmed_by:null}:{})}});
        return route.continue();
      });
      const summary=page.locator('.scorecard-summary');
      async function populated(){await expect(summary).toBeVisible();await expect(summary.locator('.scorecard-holes > li')).toHaveCount(18);await expect(summary.locator('header > strong')).toHaveText('1/18');await expect(page.getByRole('heading',{name:longName,exact:true})).toBeVisible();}
      await page.goto(url);await seen;await expect(page.getByRole('status').filter({hasText:'Laster …'}).first()).toBeVisible();await expect(summary).toHaveCount(0);
      await page.screenshot({path:out+`${kind}-loading-${width}.png`,fullPage:true});mode='pass';release();await populated();
      expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
      await page.screenshot({path:out+`${kind}-populated-${width}.png`,fullPage:true});
      mode='error';await page.reload();await expect(page.getByRole('alert')).toBeVisible();await expect(summary).toHaveCount(0);
      await page.screenshot({path:out+`${kind}-error-${width}.png`,fullPage:true});mode='pass';await page.getByRole('button',{name:'Prøv igjen',exact:true}).click();await populated();
      mode='empty';await page.reload();await expect(summary).toBeVisible();
      if(kind==='access') {await expect(page.getByText('Du kan se dette scorekortet, men ikke føre score for det.',{exact:true})).toBeVisible();await expect(summary.locator('header > strong')).toHaveText('1/18');}
      else {await expect(summary.locator('header > strong')).toHaveText('0/18');await expect(summary.locator('.scorecard-holes > li')).toHaveCount(18);}
      expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
      await page.screenshot({path:out+`${kind}-empty-${width}.png`,fullPage:true});
      mode='denied';await page.reload();await expect(page.getByRole('alert').first()).toBeVisible();await expect(summary).toHaveCount(0);
      await page.screenshot({path:out+`${kind}-denied-${width}.png`,fullPage:true});await page.unroute('**'+path);
      results.push({width,kind,actual_private_200:true,typed_decoder:true,loading:true,populated:true,long_content_no_overflow:true,error_read_only_retry:true,empty_and_denied:true,injected_responses:injected});
    }
    expect(mutations).toEqual([]);
    // Unmount the denied scorer before logout so its existing read revalidation
    // cannot be misclassified as an unexpected post-logout network failure.
    await page.goto('about:blank');expect(errors).toEqual([]);expect(unexpected).toEqual([]);
    expect((await page.request.post('/api/auth/logout',{headers:{'x-csrf-token':session.csrf_token}})).status()).toBe(204);
    for(const path of [accessPath,cardPath,scoringPath]) {const r=await page.request.get(path);expect(r.status()).toBe(401);expect(r.headers()['cache-control']).toContain('no-store');expect((await r.json()).error.code).toBe('unauthenticated');}
    await page.goto(scoreUrl);await expect(page).toHaveURL(/\/login/);expect(errors).toEqual([]);expect(unexpected).toEqual([]);
    results.push({width,logged_out_three_non_storeable_401:true,no_score_mutations:true,page_errors:errors,unexpected_network_failures:unexpected});await context.close();
  }
  console.log(JSON.stringify(results,null,2));
}finally{await browser.close();}
