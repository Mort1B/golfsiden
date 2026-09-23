const { request, chromium, expect } = require(process.cwd() + '/frontend/node_modules/@playwright/test');
const fs = require('node:fs'); const crypto = require('node:crypto'); const assert = require('node:assert/strict');
const root = '/tmp/golf-rehearsal-738ec1d';
const mode = process.argv[2]; const origin = mode !== 'verify' ? 'https://localhost:18582' : 'https://localhost:18592';
const day = new Date(Date.now()+86400000).toISOString().slice(0,10);
async function client() {return request.newContext({baseURL:origin,ignoreHTTPSErrors:true});}
async function call(c,path,data,method='GET',status=200,csrf) {
 const r=await c.fetch(path,{method,data,headers:csrf?{'x-csrf-token':csrf}:{}});
 if(r.status()!==status) throw Error(`${method} ${path} status ${r.status()}, expected ${status}`);
 return r.status()===204?null:r.json();
}
async function onboard(c,username,password,name) {
 return call(c,'/api/onboarding/tournaments',{creator:{account:{username,password},player:{display_name:name,handicap_index:12}},tournament:{name:'Recovery rehearsal '+name,description:'Synthetic deployment validation',start_date:day,end_date:day,counted_rounds:1,mandatory_round_number:null},rounds:[{round_number:1,name:'Final round',round_date:day,scoring_format:'individual_stroke_play'}]},'POST',201);
}
async function read(c,f,role) {
 const rid=f.round.id,tid=f.tournament.id;
 const paths=[`/api/tournaments/${tid}`,`/api/tournaments/${tid}/rounds`,`/api/tournaments/${tid}/players`,`/api/rounds/${rid}/leaderboards/gross`,`/api/rounds/${rid}/leaderboards/net`,`/api/tournaments/${tid}/leaderboards/gross`,`/api/tournaments/${tid}/leaderboards/net`];
 if(role==='admin') paths.push(`/api/rounds/${rid}/scorecards/player/${f.adminPlayer}/scoring`);
 const out={}; for(const p of paths) out[p]=await call(c,p); return out;
}
async function main() {
 const admin=await client(),member=await client(),outsider=await client(),anon=await client();
 let f;
 if(mode==='setup') {
  const stamp=Date.now(); const password=crypto.randomBytes(24).toString('base64url');
  f={password,adminName:`recover_admin_${stamp}`,memberName:`recover_member_${stamp}`,outsiderName:`recover_other_${stamp}`};
  const body=await onboard(admin,f.adminName,password,'Rehearsal administrator');
  f.tournament=body.tournament;f.adminPlayer=body.session.player_id;f.csrf=body.session.csrf_token;
  const reg=await call(member,`/api/invitations/${body.invitation.id}/register`,{token:body.invitation.token,account:{username:f.memberName,password},player:{display_name:'Rehearsal member',handicap_index:18}},'POST',201);
  f.memberPlayer=reg.player_id;
  await onboard(outsider,f.outsiderName,password,'Rehearsal outsider');
  const rounds=await call(admin,`/api/tournaments/${f.tournament.id}/rounds`);
  const first=Array.isArray(rounds)?rounds[0]:rounds.rounds[0];
  const mutate=(path,data={},method='POST',status=200)=>call(admin,path,data,method,status,f.csrf);
  f.round=await mutate(`/api/rounds/${first.id}/course-configuration`,{expected_round_updated_at:first.updated_at,selection:{source:'manual',course_name:'Recovery course',location:null,tee:{category:'male',name:'Yellow',course_rating:72,slope_rating:113,holes:Array.from({length:18},(_,i)=>({par:4,stroke_index:i+1,distance:null}))}}},'PUT');
  await mutate(`/api/rounds/${first.id}/pairings`,{expected_round_updated_at:f.round.updated_at,teams:[],flights:[{id:crypto.randomUUID(),name:'Recovery flight',starting_hole:1,tee_time:null,members:[{player_id:f.adminPlayer},{player_id:f.memberPlayer}]}],legacy_conversions:[]},'PUT');
  const fresh=await call(admin,`/api/tournaments/${f.tournament.id}`);
  await mutate(`/api/tournaments/${f.tournament.id}/start`,{expected_tournament_updated_at:fresh.updated_at});
  await mutate(`/api/rounds/${first.id}/open`);
  const card=await call(admin,`/api/rounds/${first.id}/scorecards/player/${f.adminPlayer}/scoring`);
  f.holes=card.holes.map(h=>h.hole_id);
  for(const [player,extra] of [[f.adminPlayer,0],[f.memberPlayer,1]]) for(let i=0;i<18;i++) await mutate(`/api/rounds/${first.id}/scores`,{owner:{type:'player',id:player},hole_id:f.holes[i],gross_strokes:4+extra+(i===17?2:0)},'PUT');
  const vis=await call(admin,`/api/tournaments/${f.tournament.id}/final-round-visibility`);
  await mutate(`/api/tournaments/${f.tournament.id}/final-round-visibility`,{back_nine_hidden:true,expected_visibility_updated_at:vis.visibility_updated_at},'PATCH');
  f.share=await mutate(`/api/tournaments/${f.tournament.id}/result-share`,{expected_grant_id:null},'POST',201);
  fs.writeFileSync(root+'/fixture-private.json',JSON.stringify(f),{mode:0o600});
 } else {
  f=JSON.parse(fs.readFileSync(root+'/fixture-private.json'));
  for(const [c,username] of [[admin,f.adminName],[member,f.memberName],[outsider,f.outsiderName]]) {
   const auth=await call(c,'/api/auth/login',{username,password:f.password},'POST');
   if(c===admin) f.csrf=auth.csrf_token;
  }
 }
 const tid=f.tournament.id;
 const results={admin:await read(admin,f,'admin'),member:await read(member,f,'member')};
 results.anonDenied=await call(anon,`/api/tournaments/${tid}`,undefined,'GET',401);
 results.outsiderDenied=await call(outsider,`/api/tournaments/${tid}`,undefined,'GET',403);
 results.public=await call(anon,`/api/public/results/${f.share.grant.id}`,{token:f.share.token,metric:'gross'},'POST');
 results.publicWithAdmin=await call(admin,`/api/public/results/${f.share.grant.id}`,{token:f.share.token,metric:'gross'},'POST');
 assert.deepEqual(results.publicWithAdmin,results.public);
 if(mode!=='verify') fs.writeFileSync(root+'/api-before-private.json',JSON.stringify(results),{mode:0o600});
 else assert.deepEqual(results,JSON.parse(fs.readFileSync(root+'/api-before-private.json')));
 console.log(`${mode}: admin/member reads, gross/net standings, scoring card, anonymous 401, outsider 403, public projection and admin-cookie projection ${mode!=='verify'?'captured':'equal before/after restore'}`);
 const card=results.admin[`/api/rounds/${f.round.id}/scorecards/player/${f.adminPlayer}/scoring`];
 assert.equal(card.gross_total,74);assert.equal(card.net_total,62);assert.equal(card.holes_scored,18);
 assert.equal(results.public.visibility.mode,'front_nine');assert.deepEqual(results.public.entries.map(e=>e.total),[36,45]);
 console.log('Exact assertions: admin gross 74/net 62/18 holes; public front nine totals 36 and 45; cookie-independent public projection.');
 await Promise.all([admin,member,outsider,anon].map(c=>c.dispose()));
}
main().catch(e=>{console.error(e.message);process.exitCode=1});
