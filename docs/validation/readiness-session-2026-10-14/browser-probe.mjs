import {chromium, expect, request} from '../../../frontend/node_modules/@playwright/test/index.mjs';
const root='http://127.0.0.1:5173', trip='00000000-0000-0000-0000-000000002001';
const first='00000000-0000-0000-0000-000000004001', second='00000000-0000-0000-0000-000000004002';
const longName='Syntetisk lag med et langt navn for kontroll av fullføring og lesbarhet på smal mobil og stor skjerm';
const browser=await chromium.launch({channel:'chrome',headless:true}), results=[];
try {
  const fixture=await request.newContext({baseURL:root});
  const login=await fixture.post('/api/auth/login',{data:{username:'admin',password:'golf-dev-2026'}});expect(login.status()).toBe(200);
  const auth=await login.json(), headers={'x-csrf-token':auth.csrf_token};
  const tournament=await(await fixture.get(`/api/tournaments/${trip}`)).json();
  if(tournament.status==='draft') {const start=await fixture.post(`/api/tournaments/${trip}/start`,{headers,data:{expected_tournament_updated_at:tournament.updated_at}});expect(start.status()).toBe(200);}
  const current=await(await fixture.get(`/api/rounds/${second}/pairings`)).json();
  if(current.status==='draft') {
  const changed=await fixture.put(`/api/rounds/${second}/pairings`,{headers,data:{expected_round_updated_at:current.updated_at,
    teams:current.teams.map((t,i)=>({id:t.id,name:i===0?longName:t.name,members:t.members.map(m=>({player_id:m.player_id})),schedule_flight_id:null})),
    flights:current.flights.map(f=>({id:f.id,name:f.name,starting_hole:f.starting_hole,tee_time:f.tee_time,members:f.members.map(m=>({player_id:m.player_id}))})),legacy_conversions:[]}});expect(changed.status()).toBe(200);
  const opened=await fixture.post(`/api/rounds/${second}/open`,{headers,data:{}});expect(opened.status()).toBe(200);
  } else {expect(current.status).toBe('open');expect(current.teams.some(t=>t.name===longName)).toBe(true);}
  await fixture.post('/api/auth/logout',{headers});await fixture.dispose();
  for(const width of [320,390,1280]) {
    const context=await browser.newContext({baseURL:root,viewport:{width,height:900}}),page=await context.newPage();
    const errors=[],unexpected=[],mutations=[],expectedFailures=new Set();
    page.on('pageerror',e=>errors.push(e.message));
    page.on('console',m=>{if(m.type()==='error'&&!m.text().startsWith('Failed to load resource:'))errors.push(m.text());});
    page.on('response',r=>{const p=new URL(r.url()).pathname;if(r.status()>=400&&!expectedFailures.has(`${r.status()} ${p}`)&&!(p==='/api/auth/session'&&r.status()===401))unexpected.push(`${r.status()} ${p}`);});
    page.on('requestfailed',r=>{if(!r.failure()?.errorText.includes('ERR_ABORTED')&&!r.url().includes('/live'))unexpected.push(new URL(r.url()).pathname);});
    page.on('request',r=>{if(r.method()!=='GET'&&/\/(open|complete|lock)$/.test(new URL(r.url()).pathname))mutations.push(r.url());});
    await page.goto('/login?returnTo=%2Ftournaments');await page.getByLabel('Brukernavn',{exact:true}).fill('admin');await page.getByLabel('Passord',{exact:true}).fill('golf-dev-2026');await page.getByRole('button',{name:'Logg inn',exact:true}).click();await expect(page).toHaveURL(/\/tournaments$/);
    const session=await(await page.request.get('/api/auth/session')).json();
    for(const [kind,id,suffix,label] of [['pairing',first,'pairing-validation','Åpne runden'],['completion',second,'completion-validation','Fullfør runden']]) {
      console.error(`${width}px ${kind} readiness`);
      const path=`/api/rounds/${id}/${suffix}`,url=`/manage/tournaments/${trip}?round=${id}#lifecycle`;
      const actual=await page.request.get(path);expect(actual.status()).toBe(200);expect(actual.headers()['cache-control']).toBe('private, no-store');const model=await actual.json();
      let release,started;const gate=new Promise(r=>release=r),seen=new Promise(r=>started=r);let mode='delay',injected=0;
      await page.route('**'+path,async route=>{
        if(mode==='delay'){started();await gate;return route.continue();}
        if(mode==='error'||mode==='denied'){const status=mode==='error'?503:403;expectedFailures.add(`${status} ${path}`);injected++;return route.fulfill({status,headers:{'cache-control':'private, no-store'},json:{error:{code:status===503?'service_unavailable':'forbidden',message:'readiness unavailable'}}});}
        if(mode==='empty')return route.fulfill({json:kind==='pairing'?{...model,ready:false,issues:[{code:'no_active_entrants',message:'round requires at least one active entrant'}],team_sizes:[],flight_sizes:[],missing_players:[],missing_flight_players:[]}:{...model,owners:[],ready_to_complete:false,ready_to_lock:false,issues:[{code:'no_required_owners',message:'round has no required scorecards'},{code:'round_not_completed',message:'round must be completed to lock'}]}});
        if(mode==='blocked')return route.fulfill({json:{...model,ready:false,issues:[{code:'missing_flight_assignment',message:'flight required'}],missing_flight_players:[{player_id:'00000000-0000-0000-0000-000000001001',display_name:longName}]}});
        return route.continue();
      });
      const panel=page.locator('.round-lifecycle'),action=panel.getByRole('button',{name:label,exact:true});
      async function populated(){await expect(panel.locator('.round-readiness')).toBeVisible();if(kind==='pairing')await expect(action).toBeEnabled();else{await expect(panel.locator('.round-completion-cards > li')).toHaveCount(4);await expect(panel.getByText(longName,{exact:true})).toBeVisible();await expect(action).toBeDisabled();}}
      await page.goto(url);await seen;await expect(panel.getByRole('status').filter({hasText:'Laster …'})).toBeVisible();await expect(action).toBeDisabled();
      await panel.screenshot({path:`/tmp/golf-a1e-${kind}-loading-${width}.png`});mode='pass';release();await populated();
      expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
      await panel.screenshot({path:`/tmp/golf-a1e-${kind}-populated-${width}.png`});
      if(kind==='pairing'){mode='blocked';await page.reload();await expect(panel.getByText(longName,{exact:true})).toBeVisible();await expect(action).toBeDisabled();expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);await panel.screenshot({path:`/tmp/golf-a1e-pairing-blocked-${width}.png`});}
      mode='error';await page.reload();await expect(panel.getByRole('alert')).toBeVisible();await expect(panel.locator('.round-readiness')).toHaveCount(0);await expect(action).toBeDisabled();await panel.screenshot({path:`/tmp/golf-a1e-${kind}-error-${width}.png`});
      mode='pass';await panel.getByRole('button',{name:'Oppdater kontrollen',exact:true}).click();await populated();
      mode='empty';await page.reload();await expect(panel.getByText(kind==='pairing'?'Minst én aktiv deltaker må være påmeldt.':'Runden har ingen nødvendige scorekort og kan ikke fullføres eller låses.',{exact:true})).toBeVisible();await expect(action).toBeDisabled();await panel.screenshot({path:`/tmp/golf-a1e-${kind}-empty-${width}.png`});
      mode='denied';await page.reload();await expect(panel.getByRole('alert')).toBeVisible();await expect(action).toBeDisabled();await expect(panel.locator('.round-readiness')).toHaveCount(0);await page.unroute('**'+path);
      results.push({width,kind,actual_private_200:true,loading:true,populated:true,long_content_no_overflow:true,error_read_only_retry:true,empty_and_denied:true,injected_responses:injected});
    }
    expect(mutations).toEqual([]);
    const logout=await page.request.post('/api/auth/logout',{headers:{'x-csrf-token':session.csrf_token}});expect(logout.status()).toBe(204);
    for(const [id,suffix] of [[first,'pairing-validation'],[second,'completion-validation']]){const r=await page.request.get(`/api/rounds/${id}/${suffix}`);expect(r.status()).toBe(401);expect(r.headers()['cache-control']).toBe('private, no-store');expect((await r.json()).error.code).toBe('unauthenticated');}
    await page.goto(`/manage/tournaments/${trip}?round=${first}#lifecycle`);await expect(page).toHaveURL(/\/login/);expect(errors).toEqual([]);expect(unexpected).toEqual([]);
    results.push({width,logged_out_both_private_401:true,no_lifecycle_mutations:mutations.length===0,page_errors:errors,unexpected_network_failures:unexpected});await context.close();
  }
  console.log(JSON.stringify(results,null,2));
}finally{await browser.close();}
