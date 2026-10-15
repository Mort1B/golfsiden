import {chromium,expect} from '../../../frontend/node_modules/@playwright/test/index.mjs';
import {fileURLToPath} from 'node:url';
const out=fileURLToPath(new URL('./',import.meta.url)),origin='http://127.0.0.1:5173';
const browser=await chromium.launch({channel:'chrome',headless:true});
async function fixture(page,hidden){
 const stamp=Date.now()+'_'+Math.floor(Math.random()*10000),day=new Date(Date.now()+86400000).toISOString().slice(0,10);
 const r=await page.request.post('/api/onboarding/tournaments',{data:{creator:{account:{username:'vis_read_'+stamp,password:'visibility-read-browser-password'},player:{display_name:'Arrangøren med et svært langt navn i turneringsoversikten',handicap_index:10}},tournament:{name:'Finaleturneringen med et svært langt navn for spillere og administratorer',description:'',start_date:day,end_date:day,counted_rounds:1,mandatory_round_number:null},rounds:[{round_number:1,name:'Finalerunden med et svært langt navn som skal forbli lesbart på telefonen',round_date:day,scoring_format:'individual_stroke_play'}]}});if(r.status()!==201)throw new Error("fixture: "+await r.text());
 const created=await r.json(),tournament=created.tournament.id,auth=created.session,path=`/api/tournaments/${tournament}/final-round-visibility`;
 const round=created.rounds[0],configured=await page.request.put(`/api/rounds/${round.id}/course-configuration`,{headers:{'x-csrf-token':auth.csrf_token},data:{expected_round_updated_at:round.updated_at,selection:{source:'manual',course_name:'Syntetisk finalebane',location:null,tee:{category:'male',name:'Gul',course_rating:72,slope_rating:113,holes:Array.from({length:18},(_,i)=>({par:4,stroke_index:i+1,distance:null}))}}}});expect(configured.status()).toBe(200);
 if(!hidden){const current=await(await page.request.get(path)).json();const r=await page.request.patch(path,{headers:{'x-csrf-token':auth.csrf_token},data:{back_nine_hidden:false,expected_visibility_updated_at:current.visibility_updated_at}});expect(r.status()).toBe(200);}
 return {tournament,path,auth,hidden};
}
const results=[];
try{
 for(const width of [320,390,1280]){
  const context=await browser.newContext({baseURL:origin,viewport:{width,height:width===1280?900:844}}),page=await context.newPage(),f=await fixture(page,width!==390);
  const errors=[],unexpected=[],mutations=[],expectedFailures=new Set();
  page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error'&&!m.text().startsWith('Failed to load resource:'))errors.push(m.text());});
  page.on('response',r=>{const p=new URL(r.url()).pathname;if(r.status()>=400&&!expectedFailures.has(`${r.status()} ${p}`)&&!(p==='/api/auth/session'&&r.status()===401))unexpected.push(`${r.status()} ${p}`);});
  page.on('requestfailed',r=>{if(!r.failure()?.errorText.includes('ERR_ABORTED')&&!r.url().includes('/live'))unexpected.push(new URL(r.url()).pathname);});
  page.on('request',r=>{if(r.method()!=='GET'&&new URL(r.url()).pathname===f.path)mutations.push(r.method());});
  await page.goto('/tournaments');
  const actual=await page.request.get(f.path);expect(actual.status()).toBe(200);expect(actual.headers()['cache-control']).toBe('private, no-store');const model=await actual.json();expect(Object.keys(model).sort()).toEqual(['tournament_id','back_nine_hidden','visibility_updated_at'].sort());expect(model.back_nine_hidden).toBe(f.hidden);
  const decoded=await page.evaluate(async({tournament})=>{const {finalRoundVisibilityApi}=await import('/src/api/finalRoundVisibility.ts');return finalRoundVisibilityApi.get(tournament);},f);expect(decoded).toEqual(model);
  let release,started=false;const gate=new Promise(r=>release=r);let mode='delay',injected=0;
  await page.route('**'+f.path,async route=>{
   if(mode==='delay'){started=true;await gate;return route.continue();}
   if(mode!=='pass'){const status=Number(mode);expectedFailures.add(`${status} ${f.path}`);injected++;return route.fulfill({status,headers:{'cache-control':'private, no-store'},json:{error:{code:status===503?'service_unavailable':status===500?'internal_error':status===404?'not_found':'forbidden',message:'Synlighetsstatus er utilgjengelig'}}});}
   return route.continue();
  });
  const url=`/manage/tournaments/${f.tournament}#lifecycle`,control=page.locator('.final-visibility-control');
  async function populated(){const toggle=control.getByRole('switch',{name:'Frigi hull 10–18'});await expect(toggle).toBeEnabled();if(f.hidden)await expect(toggle).not.toBeChecked();else await expect(toggle).toBeChecked();await expect(control.getByText(f.hidden?'Hull 10–18 er skjult':'Hull 10–18 er frigitt',{exact:false})).toBeVisible();await control.scrollIntoViewIfNeeded();expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);expect((await control.locator('.final-visibility-switch').boundingBox()).height).toBeGreaterThanOrEqual(44);}
  await page.goto(url);await expect.poll(()=>started,{timeout:15000}).toBe(true);await expect(control.getByRole('status')).toHaveText('Henter synlighetsstatus …');await expect(control.getByRole('switch')).toHaveCount(0);mode='pass';release();await populated();
  if(width===390)await page.screenshot({path:out+'visibility-released-390.png'});
  if(width===1280)await page.screenshot({path:out+'visibility-hidden-1280.png'});
  for(const status of [500,503]){mode=String(status);await page.reload();await expect(control).toHaveAttribute('role','alert');await expect(control.getByRole('switch')).toHaveCount(0);await control.scrollIntoViewIfNeeded();if(width===320&&status===503)await page.screenshot({path:out+'visibility-error-320.png'});mode='pass';await control.getByRole('button',{name:'Prøv igjen',exact:true}).click();await populated();}
  for(const status of [403,404]){mode=String(status);await page.reload();await expect(control).toHaveAttribute('role','alert');await expect(control.getByRole('switch')).toHaveCount(0);}
  const missing=await page.request.get('/api/tournaments/00000000-0000-0000-0000-000000000099/final-round-visibility');expect(missing.status()).toBe(404);expect(missing.headers()['cache-control']).toBe('private, no-store');expect((await missing.json()).error.code).toBe('not_found');
  const invalid=await page.request.get('/api/tournaments/invalid/final-round-visibility');expect(invalid.status()).toBe(400);expect(invalid.headers()['cache-control']).toBe('private, no-store');
  await page.goto('about:blank');await page.unroute('**'+f.path);expect(mutations).toEqual([]);expect(errors).toEqual([]);expect(unexpected).toEqual([]);
  expect((await page.request.post('/api/auth/logout',{headers:{'x-csrf-token':f.auth.csrf_token}})).status()).toBe(204);
  const loggedOut=await page.request.get(f.path);expect(loggedOut.status()).toBe(401);expect(loggedOut.headers()['cache-control']).toBe('private, no-store');expect((await loggedOut.json()).error.code).toBe('unauthenticated');
  await page.goto(url);await expect(page).toHaveURL(/\/login/);expect(errors).toEqual([]);expect(unexpected).toEqual([]);
  results.push({width,hidden:f.hidden,actual_private_200:true,typed_decoder_full_json_and_version:true,loading_without_control:true,long_content_no_overflow:true,touch_target_44px:true,private_500_503_read_only_retry:true,initial_403_404_without_control:true,actual_missing404_invalid400_private:true,logged_out_private401:true,no_visibility_mutations_during_observation:true,empty_collection_state:'not applicable: singleton three-field DTO',page_errors:errors,unexpected_network_failures:unexpected,injected_responses:injected});await context.close();
 }
 console.log(JSON.stringify(results,null,2));
}finally{await browser.close();}
