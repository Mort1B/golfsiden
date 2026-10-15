import {chromium,expect} from '../../../frontend/node_modules/@playwright/test/index.mjs';
import {fileURLToPath} from 'node:url';
const out=fileURLToPath(new URL('./',import.meta.url)),origin='http://127.0.0.1:5173',password='invitation-read-browser-password';
const browser=await chromium.launch({channel:'chrome',headless:true});
async function fixture(page){
 const stamp=Date.now()+'_'+Math.floor(Math.random()*10000),day=new Date(Date.now()+86400000).toISOString().slice(0,10),name='Invitasjonsturneringen med et svært langt navn for spillere og administratorer';
 const r=await page.request.post('/api/onboarding/tournaments',{data:{creator:{account:{username:'invite_read_'+stamp,password},player:{display_name:'Arrangøren med et svært langt navn i invitasjonsoversikten',handicap_index:10}},tournament:{name,description:'',start_date:day,end_date:day,counted_rounds:1,mandatory_round_number:null},rounds:[{round_number:1,name:'Runden med et langt navn',round_date:day,scoring_format:'individual_stroke_play'}]}});expect(r.status()).toBe(201);
 const created=await r.json(),tournament=created.tournament.id,auth=created.session,path=`/api/tournaments/${tournament}/invitations`;
 async function issue(max_uses){const r=await page.request.post(path,{headers:{'x-csrf-token':auth.csrf_token},data:{expires_at:new Date(Date.now()+86400000).toISOString(),max_uses}});expect(r.status()).toBe(201);return r.json();}
 const used=await issue(1),other=await browser.newContext({baseURL:origin});
 try{const r=await other.request.post(`/api/invitations/${used.id}/register`,{data:{token:used.token,account:{username:'invite_joined_'+stamp,password},player:{display_name:'Syntetisk påmeldt spiller',handicap_index:12}}});expect(r.status()).toBe(201);}finally{await other.close();}
 const revoked=await issue(5);expect((await page.request.delete(`${path}/${revoked.id}`,{headers:{'x-csrf-token':auth.csrf_token}})).status()).toBe(204);
 return {tournament,path,name};
}
const results=[];
try{
 for(const width of [320,390,1280]){
  const context=await browser.newContext({baseURL:origin,viewport:{width,height:width===1280?900:844}}),page=await context.newPage(),f=await fixture(page);
  const errors=[],unexpected=[],mutations=[],expectedFailures=new Set();
  page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error'&&!m.text().startsWith('Failed to load resource:'))errors.push(m.text());});
  page.on('response',r=>{const p=new URL(r.url()).pathname;if(r.status()>=400&&!expectedFailures.has(`${r.status()} ${p}`)&&!(p==='/api/auth/session'&&r.status()===401))unexpected.push(`${r.status()} ${p}`);});
  page.on('requestfailed',r=>{if(!r.failure()?.errorText.includes('ERR_ABORTED')&&!r.url().includes('/live'))unexpected.push(new URL(r.url()).pathname);});
  page.on('request',r=>{if(r.method()!=='GET'&&(new URL(r.url()).pathname.startsWith('/api/tournaments/')||new URL(r.url()).pathname.startsWith('/api/invitations/')))mutations.push(r.method()+' '+new URL(r.url()).pathname);});
  await page.goto('/tournaments');const auth=await(await page.request.get('/api/auth/session')).json();
  const decoded=await page.evaluate(async({tournament})=>{const {listInvitations}=await import('/src/api/invitations.ts');const values=await listInvitations(tournament);return {rows:values.length,counts:values.map(v=>v.redemption_count).sort(),keys:Object.keys(values[0]).sort()};},f);
  expect(decoded.rows).toBe(3);expect(decoded.counts).toEqual([0,0,1]);expect(decoded.keys).toEqual(['id','tournament_id','series_id','predecessor_id','created_by_user_id','created_at','expires_at','revoked_at','revoked_by_user_id','revocation_actor_known','max_uses','redemption_count'].sort());
  const actual=await page.request.get(f.path);expect(actual.status()).toBe(200);expect(actual.headers()['cache-control']).toBe('no-store');const model=await actual.json();expect(model.every(i=>!Object.hasOwn(i,'token')&&!Object.hasOwn(i,'token_hash'))).toBe(true);
  let release,started;const gate=new Promise(r=>release=r),seen=new Promise(r=>started=r);let mode='delay',injected=0;
  await page.route('**'+f.path,async route=>{
   if(mode==='delay'){started();await gate;return route.continue();}
   if(mode==='error'||mode==='denied'){const status=mode==='error'?500:403;expectedFailures.add(`${status} ${f.path}`);injected++;return route.fulfill({status,headers:{'cache-control':'no-store'},json:{error:{code:status===500?'internal_error':'forbidden',message:'Invitation metadata unavailable'}}});}
   if(mode==='empty')return route.fulfill({headers:{'cache-control':'no-store'},json:[]});
   return route.continue();
  });
  const url=`/tournaments/${f.tournament}/invitations`,section=page.getByRole('region',{name:'Invitasjonslenker',exact:true}),list=page.locator('ul.invitation-list');
  async function populated(){await expect(list.locator(':scope > li')).toHaveCount(3);for(const status of ['Aktiv','Brukt opp','Tilbakekalt'])await expect(list.locator('.invitation-status').filter({hasText:new RegExp('^'+status+'$')})).toBeVisible();await expect(list.getByText('1 av 1 påmeldt',{exact:true})).toBeVisible();await expect(page.getByRole('heading',{name:'Invitasjoner',exact:true})).toBeVisible();}
  await page.goto(url);await seen;await expect(section.getByRole('status').filter({hasText:'Laster …'})).toBeVisible();await expect(list).toHaveCount(0);mode='pass';release();await populated();expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
  await expect(page.getByRole('button',{name:'Kopier lenke',exact:true})).toHaveCount(0);
  if(width===390){await list.locator(':scope > li').first().scrollIntoViewIfNeeded();await page.screenshot({path:out+'metadata-populated-390.png'});}
  mode='error';await page.reload();await expect(section.getByRole('alert')).toBeVisible();await expect(list).toHaveCount(0);
  if(width===320){await section.scrollIntoViewIfNeeded();await expect(section.getByRole('alert')).toBeInViewport();await page.screenshot({path:out+'metadata-error-320.png'});}
  mode='pass';await section.getByRole('button',{name:'Prøv igjen',exact:true}).click();await populated();
  mode='empty';await page.reload();await expect(section.getByText('Ingen invitasjoner er opprettet.',{exact:true})).toBeVisible();await expect(list).toHaveCount(0);expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
  if(width===1280){await section.scrollIntoViewIfNeeded();await page.screenshot({path:out+'metadata-empty-1280.png'});}
  mode='denied';await page.reload();await expect(section.getByRole('alert')).toBeVisible();await expect(list).toHaveCount(0);
  await page.goto('about:blank');await page.unroute('**'+f.path);expect(mutations).toEqual([]);expect(errors).toEqual([]);expect(unexpected).toEqual([]);
  expect((await page.request.post('/api/auth/logout',{headers:{'x-csrf-token':auth.csrf_token}})).status()).toBe(204);
  const loggedOut=await page.request.get(f.path);expect(loggedOut.status()).toBe(401);expect(loggedOut.headers()['cache-control']).toBe('no-store');expect((await loggedOut.json()).error.code).toBe('unauthenticated');
  await page.goto(url);await expect(page).toHaveURL(/\/login/);expect(errors).toEqual([]);expect(unexpected).toEqual([]);
  results.push({width,actual_no_store_200:true,typed_decoder:true,safe_metadata_fields:true,loading:true,populated_active_exhausted_revoked:true,long_content_no_overflow:true,error_read_only_retry:true,empty_and_initial_denied:true,logged_out_no_store_401:true,no_invitation_mutations:true,page_errors:errors,unexpected_network_failures:unexpected,injected_responses:injected});await context.close();
 }
 console.log(JSON.stringify(results,null,2));
}finally{await browser.close();}
