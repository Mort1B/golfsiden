import {chromium, expect, request} from '../../../frontend/node_modules/@playwright/test/index.mjs';

const root='http://127.0.0.1:5173';
const username=`a1h_${Date.now()}`,password='a1h-browser-password';
const longName='Syntetisk four-ball-lag med et langt navn for sikre scorekort på smal mobil og stor skjerm';
const names=['Syntetisk partner A med et langt navn for mobilvisning','Syntetisk partner B med ekstra slag og et langt navn'];
const out='/tmp/golf-a1h-';
const browser=await chromium.launch({channel:'chrome',headless:true}),results=[];
try {
  const fixture=await request.newContext({baseURL:root});
  const day=new Date(Date.now()+86400000).toISOString().slice(0,10);
  const created=await fixture.post('/api/onboarding/tournaments',{data:{
    creator:{account:{username,password},player:{display_name:names[0],handicap_index:0}},
    tournament:{name:'Syntetisk A1h four-ball',description:'',start_date:day,end_date:day,counted_rounds:1,mandatory_round_number:null},
    rounds:[{round_number:1,name:'Syntetisk four-ball-runde',round_date:day,scoring_format:'four_ball_stroke_play'}]}});
  expect(created.status()).toBe(201);const setup=await created.json(),trip=setup.tournament.id,auth=setup.session,headers={'x-csrf-token':auth.csrf_token};
  const partner=await request.newContext({baseURL:root});
  const joined=await partner.post(`/api/invitations/${setup.invitation.id}/register`,{data:{token:setup.invitation.token,account:{username:`partner_${Date.now()}`,password},player:{display_name:names[1],handicap_index:21.2}}});
  expect(joined.status()).toBe(201);const second=(await joined.json()).player_id;await partner.dispose();
  const first=(await(await fixture.get(`/api/tournaments/${trip}/rounds`)).json())[0],round=first.id;
  const configured=await fixture.put(`/api/rounds/${round}/course-configuration`,{headers,data:{expected_round_updated_at:first.updated_at,selection:{source:'manual',course_name:'Syntetisk A1h bane',location:null,tee:{category:'male',name:'Test',course_rating:72,slope_rating:113,holes:Array.from({length:18},(_,i)=>({par:4,stroke_index:i+1,distance:null}))}}}});
  expect(configured.status()).toBe(200);const current=await configured.json(),team=crypto.randomUUID(),members=[{player_id:auth.player_id},{player_id:second}];
  expect((await fixture.put(`/api/rounds/${round}/pairings`,{headers,data:{expected_round_updated_at:current.updated_at,teams:[{id:team,name:longName,members,schedule_flight_id:null}],flights:[{id:crypto.randomUUID(),name:'Syntetisk flight',starting_hole:1,tee_time:null,members}],legacy_conversions:[]}})).status()).toBe(200);
  const tournament=await(await fixture.get(`/api/tournaments/${trip}`)).json();
  expect((await fixture.post(`/api/tournaments/${trip}/start`,{headers,data:{expected_tournament_updated_at:tournament.updated_at}})).status()).toBe(200);
  expect((await fixture.post(`/api/rounds/${round}/open`,{headers,data:{}})).status()).toBe(200);
  const cardPath=`/api/rounds/${round}/four-ball/scorecards/${team}`,scoringPath=cardPath+'/scoring';
  const initial=await(await fixture.get(scoringPath)).json();
  for(const [hole,player,input] of [[0,auth.player_id,{type:'numeric',gross_strokes:5}],[0,second,{type:'numeric',gross_strokes:5}],[1,auth.player_id,{type:'no_score'}]]) {
    expect((await fixture.put(`/api/rounds/${round}/four-ball/inputs/conditional`,{headers,data:{request_id:crypto.randomUUID(),hole_id:initial.holes[hole].hole_id,owner:{type:'player',id:player},input,expected_score:{type:'absent'}}})).status()).toBe(200);
  }
  const scoreUrl=`/score?tournament=${trip}&round=${round}&owner_type=team&owner=${team}&view=summary`;
  const memberUrl=`/tournaments/${trip}/rounds/${round}/scorecards/team/${team}?metric=gross&view=summary`;
  expect((await fixture.post('/api/auth/logout',{headers})).status()).toBe(204);await fixture.dispose();
  for(const width of [320,390,1280]) {
    const context=await browser.newContext({baseURL:root,viewport:{width,height:900}}),page=await context.newPage();
    const errors=[],unexpected=[],mutations=[],expectedFailures=new Set();
    page.on('pageerror',e=>errors.push(e.message));
    page.on('console',m=>{if(m.type()==='error'&&!m.text().startsWith('Failed to load resource:'))errors.push(m.text());});
    page.on('response',r=>{const p=new URL(r.url()).pathname;if(r.status()>=400&&!expectedFailures.has(`${r.status()} ${p}`)&&!(p==='/api/auth/session'&&r.status()===401))unexpected.push(`${r.status()} ${p}`);});
    page.on('requestfailed',r=>{if(!r.failure()?.errorText.includes('ERR_ABORTED')&&!r.url().includes('/live'))unexpected.push(new URL(r.url()).pathname);});
    page.on('request',r=>{const p=new URL(r.url()).pathname;if(r.method()!=='GET'&&p.startsWith('/api/rounds/'))mutations.push(`${r.method()} ${p}`);});
    await page.goto('/login?returnTo=%2Ftournaments');await page.getByLabel('Brukernavn',{exact:true}).fill(username);await page.getByLabel('Passord',{exact:true}).fill(password);await page.getByRole('button',{name:'Logg inn',exact:true}).click();await expect(page).toHaveURL(/\/tournaments$/);
    const session=await(await page.request.get('/api/auth/session')).json();
    const decoded=await page.evaluate(async({round,team})=>{
      const {fourBallApi}=await import('/src/api/fourBall.ts');
      const [member,scoring]=await Promise.all([fourBallApi.read(round,team),fourBallApi.scoring(round,team)]);
      return {member:member.projection,scoring:scoring.projection,holes:member.holes.length,scored:scoring.holes_scored,winners:scoring.holes[0].gross.player_ids.length,pickup:member.holes[1].players[0].score.input.type,unentered:member.holes[1].players[1].score};
    },{round,team});
    expect(decoded).toEqual({member:'read',scoring:'scoring',holes:18,scored:1,winners:2,pickup:'no_score',unentered:null});
    for(const [kind,path,url] of [['member',cardPath,memberUrl],['scoring',scoringPath,scoreUrl]]) {
      console.error(`${width}px ${kind} read`);
      const actual=await page.request.get(path);expect(actual.status()).toBe(200);expect(actual.headers()['cache-control']).toBe('private, no-store');const model=await actual.json();
      let release,started;const gate=new Promise(r=>release=r),seen=new Promise(r=>started=r);let mode='delay',injected=0;
      await page.route('**'+path,async route=>{
        if(mode==='delay'){started();await gate;return route.continue();}
        if(mode==='error'||mode==='denied'){const status=mode==='error'?503:403;expectedFailures.add(`${status} ${path}`);injected++;return route.fulfill({status,headers:{'cache-control':'private, no-store'},json:{error:{code:status===503?'service_unavailable':'forbidden',message:'score read unavailable'}}});}
        if(mode==='empty')return route.fulfill({headers:{'cache-control':'private, no-store'},json:{...model,holes:model.holes.map(h=>({...h,players:h.players.map(p=>({...p,score:null,net_strokes:null})),gross:null,net:null})),holes_scored:0,gross_total:null,net_total:null,par_played:0,complete:false,confirmed:false,confirmed_at:null}});
        return route.continue();
      });
      const summary=page.locator('.four-ball-summary');
      async function populated(){await expect(summary).toBeVisible();await expect(summary.locator('ol > li')).toHaveCount(18);await expect(summary.locator('dl.scorecard-strip').getByText('1/18',{exact:true})).toHaveText('1/18');await expect(page.getByRole('heading',{name:longName,exact:true})).toBeVisible();}
      await page.goto(url);await seen;await expect(page.getByRole('status').filter({hasText:'Laster …'}).first()).toBeVisible();await expect(summary).toHaveCount(0);
      await page.screenshot({path:out+`${kind}-loading-${width}.png`,fullPage:false});mode='pass';release();await populated();
      expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
      await page.screenshot({path:out+`${kind}-populated-${width}.png`,fullPage:false});
      mode='error';await page.reload();await expect(page.getByRole('alert')).toBeVisible();await expect(summary).toHaveCount(0);
      await page.screenshot({path:out+`${kind}-error-${width}.png`,fullPage:false});mode='pass';await page.getByRole('button',{name:'Prøv igjen',exact:true}).click();await populated();
      mode='empty';await page.reload();await expect(summary).toBeVisible();
      await expect(summary.locator('dl.scorecard-strip').getByText('0/18',{exact:true})).toBeVisible();await expect(summary.locator('ol > li')).toHaveCount(18);
      expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
      await page.screenshot({path:out+`${kind}-empty-${width}.png`,fullPage:false});
      mode='denied';await page.reload();await expect(page.getByRole('alert').first()).toBeVisible();await expect(summary).toHaveCount(0);
      await page.screenshot({path:out+`${kind}-denied-${width}.png`,fullPage:false});await page.unroute('**'+path);
      results.push({width,kind,actual_private_200:true,typed_decoder:true,loading:true,populated:true,long_content_no_overflow:true,error_read_only_retry:true,empty_and_denied:true,injected_responses:injected});
    }
    expect(mutations).toEqual([]);
    // Unmount the denied scorer before logout so its existing read revalidation
    // cannot be misclassified as an unexpected post-logout network failure.
    await page.goto('about:blank');expect(errors).toEqual([]);expect(unexpected).toEqual([]);
    expect((await page.request.post('/api/auth/logout',{headers:{'x-csrf-token':session.csrf_token}})).status()).toBe(204);
    for(const path of [cardPath,scoringPath]) {const r=await page.request.get(path);expect(r.status()).toBe(401);expect(r.headers()['cache-control']).toContain('no-store');expect((await r.json()).error.code).toBe('unauthenticated');}
    await page.goto(scoreUrl);await expect(page).toHaveURL(/\/login/);expect(errors).toEqual([]);expect(unexpected).toEqual([]);
    results.push({width,logged_out_both_non_storeable_401:true,no_score_mutations:true,page_errors:errors,unexpected_network_failures:unexpected});await context.close();
  }
  console.log(JSON.stringify(results,null,2));
}finally{await browser.close();}
