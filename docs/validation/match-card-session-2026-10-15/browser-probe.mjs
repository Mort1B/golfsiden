import {chromium,expect} from '../../../frontend/node_modules/@playwright/test/index.mjs';
import {fileURLToPath} from 'node:url';
const out=fileURLToPath(new URL('./',import.meta.url));
const browser=await chromium.launch({channel:'chrome',headless:true});
const origin='http://127.0.0.1:5173',password='match-browser-password';
async function fixture(page) {
 const stamp=Date.now()+'_'+Math.floor(Math.random()*10000), day=new Date(Date.now()+86400000).toISOString().slice(0,10),username='match_read_'+stamp;
 const first='Andreas med et svært langt navn i matchspillturneringen',second='Bjørn motstander med et annet svært langt navn';
 const r=await page.request.post('/api/onboarding/tournaments',{data:{creator:{account:{username,password},player:{display_name:first,handicap_index:0}},tournament:{name:'Match read '+stamp,description:'',start_date:day,end_date:day,counted_rounds:null,mandatory_round_number:null},rounds:[{round_number:1,name:'Matchspill med lange navn',round_date:day,scoring_format:'singles_match_play'}]}});expect(r.status(),await r.text()).toBe(201);
 const created=await r.json(),auth=created.session,tournament=created.tournament;
 const other=await browser.newContext({baseURL:origin});let secondId;
 try {const r=await other.request.post(`/api/invitations/${created.invitation.id}/register`,{data:{token:created.invitation.token,account:{username:'opponent_'+stamp,password},player:{display_name:second,handicap_index:0}}});expect(r.status(),await r.text()).toBe(201);secondId=(await r.json()).player_id;}finally{await other.close();}
 const rounds=await(await page.request.get(`/api/tournaments/${tournament.id}/rounds`)).json();let round=rounds[0];
 async function mutate(path,data={},method='POST'){const r=await page.request.fetch(path,{method,data,headers:{'x-csrf-token':auth.csrf_token}});expect(r.ok(),`${path}: ${r.status()} ${await r.text()}`).toBe(true);return r;}
 round=await(await mutate(`/api/rounds/${round.id}/course-configuration`,{expected_round_updated_at:round.updated_at,selection:{source:'manual',course_name:'Match testbane',location:null,tee:{category:'male',name:'Gul',course_rating:72,slope_rating:113,holes:Array.from({length:18},(_,i)=>({par:4,stroke_index:i+1,distance:null}))}}},'PUT')).json();
 await mutate(`/api/rounds/${round.id}/pairings`,{expected_round_updated_at:round.updated_at,teams:[],flights:[{id:crypto.randomUUID(),name:'Flight 1',starting_hole:1,tee_time:null,members:[{player_id:auth.player_id},{player_id:secondId}]}],legacy_conversions:[]},'PUT');
 round=await(await page.request.get(`/api/rounds/${round.id}`)).json();
 const listing=await(await mutate(`/api/rounds/${round.id}/match-play/matches`,{expected_round_updated_at:round.updated_at,matches:[{first_player_id:auth.player_id,second_player_id:secondId}]},'PUT')).json(),match=listing.matches[0].match_id;
 const fresh=await(await page.request.get(`/api/tournaments/${tournament.id}`)).json();await mutate(`/api/tournaments/${tournament.id}/start`,{expected_tournament_updated_at:fresh.updated_at});await mutate(`/api/rounds/${round.id}/open`);
 const path=`/api/rounds/${round.id}/match-play/matches/${match}`;
 for(const [revision,command] of [['1',{type:'note',player_id:auth.player_id,hole_number:1,gross_strokes:4}],['2',{type:'report',event:{type:'hole',hole_number:1,outcome:'halved',basis:{type:'agreed_halve',play_begun:true,mutual_agreement:true}}}]])await mutate(path+'/commands',{request_id:crypto.randomUUID(),expected_revision:revision,command});
 return {round:round.id,match,path,first,second,username};
}
const results=[];
try {
 for(const width of [320,390,1280]) {
  const context=await browser.newContext({baseURL:origin,viewport:{width,height:width===1280?900:844}}),page=await context.newPage();
  const f=await fixture(page),errors=[],unexpected=[],mutations=[],expectedFailures=new Set();
  page.on('pageerror',e=>errors.push(e.message));
  page.on('console',m=>{if(m.type()==='error'&&!m.text().startsWith('Failed to load resource:'))errors.push(m.text());});
  page.on('response',r=>{const path=new URL(r.url()).pathname;if(r.status()>=400&&!expectedFailures.has(`${r.status()} ${path}`)&&!(path==='/api/auth/session'&&r.status()===401))unexpected.push(`${r.status()} ${path}`);});
  page.on('requestfailed',r=>{if(!r.failure()?.errorText.includes('ERR_ABORTED')&&!r.url().includes('/live'))unexpected.push(new URL(r.url()).pathname);});
  page.on('request',r=>{if(r.method()!=='GET'&&new URL(r.url()).pathname.startsWith('/api/rounds/'))mutations.push(r.method()+' '+new URL(r.url()).pathname);});
  await page.goto('/tournaments');await expect(page).toHaveURL(/\/tournaments$/);
  const auth=await(await page.request.get('/api/auth/session')).json();
  const decoded=await page.evaluate(async({round,match})=>{const {matchApi}=await import('/src/api/matchPlay.ts');const [member,scoring]=await Promise.all([matchApi.read(round,match),matchApi.scoring(round,match)]);return {member:member.events.length,scoring:scoring.accepted_events.length,revision:scoring.revision,holes:member.holes.length,note:member.notes[0].gross_strokes,lead:member.lead};},f);
  expect(decoded).toEqual({member:1,scoring:1,revision:'3',holes:18,note:4,lead:0});
  for(const [kind,path,url] of [['member',f.path,`/rounds/${f.round}/matches/${f.match}`],['scoring',f.path+'/scoring',`/rounds/${f.round}/matches/${f.match}/score`]]) {
   console.error(`${width}px ${kind} read`);
   const actual=await page.request.get(path);expect(actual.status()).toBe(200);expect(actual.headers()['cache-control']).toBe('private, no-store');const model=await actual.json();
   let release,started;const gate=new Promise(r=>release=r),seen=new Promise(r=>started=r);let mode='delay',injected=0;
   await page.route('**'+path,async route=>{
    if(mode==='delay'){started();await gate;return route.continue();}
    if(mode==='error'||mode==='denied'){const status=mode==='error'?503:403;expectedFailures.add(`${status} ${path}`);injected++;return route.fulfill({status,headers:{'cache-control':'private, no-store'},json:{error:{code:status===503?'service_unavailable':'forbidden',message:'Match read unavailable'}}});}
    if(mode==='empty')return route.fulfill({headers:{'cache-control':'private, no-store'},json:{...model,events:[],notes:[],resolved_holes:0,lead:0,finish:null,confirmed:false,correction_pending:false,half_points:null,...(kind==='scoring'?{accepted_events:[]}:{} )}});
    return route.continue();
   });
   const panel=page.locator('section.match-panel').filter({has:page.getByRole('heading',{name:f.first+' mot '+f.second,exact:true})});
   async function populated(){await expect(panel).toBeVisible();await expect(panel.getByText('Lik etter 1 hull',{exact:true})).toBeVisible();}
   await page.goto(url);await seen;await expect(page.getByRole('status').filter({hasText:'Laster …'}).first()).toBeVisible();await expect(panel).toHaveCount(0);
   mode='pass';release();await populated();expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
   if(width===390&&kind==='scoring')await page.screenshot({path:out+'scoring-populated-390.png'});
   mode='error';await page.reload();await expect(page.getByRole('alert').first()).toBeVisible();await expect(panel).toHaveCount(0);
   if(width===320&&kind==='member')await page.screenshot({path:out+'member-error-320.png'});
   mode='pass';await page.getByRole('button',{name:'Prøv igjen',exact:true}).click();await populated();
   mode='empty';await page.reload();await expect(panel).toBeVisible();await expect(panel.getByText('Ikke startet',{exact:true})).toBeVisible();await expect(panel.getByText('Ingen hull eller matchresultat er rapportert.',{exact:true})).toBeVisible();
   expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
   if(width===1280&&kind==='member')await page.screenshot({path:out+'member-empty-1280.png'});
   mode='denied';await page.reload();await expect(page.getByRole('alert').first()).toBeVisible();await expect(panel).toHaveCount(0);await page.unroute('**'+path);
   results.push({width,kind,actual_private_200:true,typed_decoder:true,loading:true,populated:true,long_content_no_overflow:true,error_read_only_retry:true,empty_and_initial_denied:true,injected_responses:injected});
  }
  expect(mutations).toEqual([]);await page.goto('about:blank');expect(errors).toEqual([]);expect(unexpected).toEqual([]);
  expect((await page.request.post('/api/auth/logout',{headers:{'x-csrf-token':auth.csrf_token}})).status()).toBe(204);
  for(const path of [f.path,f.path+'/scoring']){const r=await page.request.get(path);expect(r.status()).toBe(401);expect(r.headers()['cache-control']).toBe('private, no-store');expect((await r.json()).error.code).toBe('unauthenticated');}
  await page.goto(`/rounds/${f.round}/matches/${f.match}/score`);await expect(page).toHaveURL(/\/login/);expect(errors).toEqual([]);expect(unexpected).toEqual([]);
  results.push({width,logged_out_both_private_401:true,no_match_mutations:true,page_errors:errors,unexpected_network_failures:unexpected});await context.close();
 }
 console.log(JSON.stringify(results,null,2));
}finally{await browser.close();}
