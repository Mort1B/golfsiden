const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const { createRequire } = require('node:module');
const { chromium, expect } = createRequire(path.join(process.cwd(), 'frontend/package.json'))('@playwright/test');
const control = JSON.parse(fs.readFileSync(process.argv[2]));
const fixture = JSON.parse(fs.readFileSync(path.join(control.directory, 'critical-private.json')));
const replay = JSON.parse(fs.readFileSync(path.join(control.directory, 'recovery-lineup-replay.json')));
const postDatabase = process.argv[3] === 'post-database';
const evidenceDirectory = process.env.GOLF_RECOVERY_EVIDENCE_DIRECTORY || __dirname;
const livePath = `/api/tournaments/${fixture.tid}/live`;
const base = `/api/tournaments/${fixture.tid}/fantasy`;
const errors = [], pending = [], checks = {};
let outage = false;
const pass = name => { checks[name] = true; console.log(JSON.stringify({ check:name, passed:true })); };
async function read(page, endpoint, status=200) {
  const response = await page.request.get(endpoint); assert.equal(response.status(), status);
  return response.json();
}
async function login(page, username) {
  await page.goto('/login');
  await page.getByLabel('Brukernavn', {exact:true}).fill(username);
  await page.getByLabel('Passord', {exact:true}).fill(fixture.password);
  await page.getByRole('button', {name:'Logg inn',exact:true}).click();
  await expect(page).not.toHaveURL(/\/login/);
  return read(page,'/api/auth/session');
}
async function facts(page, org, member) {
  const rid=fixture.rounds[0].id;
  const card=await read(page,`/api/rounds/${rid}/scorecards/player/${fixture.players[0]}`);
  assert.equal(card.gross_total,73); assert.equal(card.net_total,73); assert.equal(card.confirmed,true);
  assert.equal((await read(page,`/api/rounds/${rid}`)).status,'locked');
  const results=await read(page,base+'/results');
  assert.deepEqual(results.managers.find(value=>value.id===org.user_id).points,{state:'settled',total:88});
  assert.deepEqual(results.managers.find(value=>value.id===member.user_id).points,{state:'settled',total:94});
  const final=await read(page,base+`/rounds/${fixture.rounds[1].id}`);
  const selection=final.selections.find(value=>value.user_id===member.user_id);
  assert.equal(selection.receipt.origin,'carried_forward'); assert.equal(selection.receipt.source_round,rid);
  const visible=await read(page,`/api/rounds/${fixture.rounds[1].id}/scorecards/player/${fixture.players[0]}`);
  assert.equal(visible.visibility.mode,'full'); assert.equal(visible.visible_hole_count,18);
  return card;
}
async function main() {
  const browser=await chromium.launch({channel:'chrome',headless:true}); const contexts=[];
  try {
    async function page() {
      const context=await browser.newContext({baseURL:control.origin,ignoreHTTPSErrors:true,viewport:{width:390,height:900}});
      contexts.push(context); const page=await context.newPage();
      page.on('pageerror',()=>errors.push('pageerror'));
      page.on('console',message=>{if(message.type()==='error'&&!message.text().startsWith('Failed to load resource:'))errors.push('console error');});
      page.on('requestfailed',req=>{
        const endpoint=new URL(req.url()).pathname, failure=req.failure()?.errorText;
        if(req.method()==='GET'&&failure==='net::ERR_ABORTED')return;
        if(outage&&req.method()==='GET'&&endpoint===livePath&&['net::ERR_CONNECTION_CLOSED','net::ERR_CONNECTION_REFUSED','net::ERR_EMPTY_RESPONSE','net::ERR_HTTP2_PROTOCOL_ERROR','net::ERR_NETWORK_CHANGED'].includes(failure))return;
        errors.push('unexpected request failure');
      });
      page.on('response',response=>{
        if(response.status()<400)return;
        const endpoint=new URL(response.url()).pathname;
        if(response.status()===401&&endpoint==='/api/auth/session')return;
        if(outage&&endpoint===livePath&&[502,503].includes(response.status()))return;
        const locked=fixture.rounds.some(round=>fixture.players.some(player=>endpoint===`/api/rounds/${round.id}/scorecards/player/${player}/scoring`));
        pending.push((async()=>{assert(locked&&response.status()===403);assert.equal((await response.json()).error.code,'forbidden');})().catch(()=>errors.push('unexpected HTTP/body')));
      });
      return page;
    }
    const organizer=await page(),member=await page();
    const org=await login(organizer,fixture.username),player=await login(member,fixture.memberName);
    assert.notEqual(org.user_id,player.user_id);
    const card=await facts(member,org,player);
    const repeated=await organizer.request.put(replay.request.path,{data:replay.request.body,headers:{'x-csrf-token':org.csrf_token}});
    assert.equal(repeated.status(),200);assert.deepEqual(await repeated.json(),replay.receipt);
    const denied=await organizer.request.put(`/api/rounds/${fixture.rounds[0].id}/scores`,{data:{owner:{type:'player',id:fixture.players[0]},hole_id:card.holes[0].hole_id,gross_strokes:9},headers:{'x-csrf-token':org.csrf_token}});
    assert.equal(denied.status(),409);assert.equal((await denied.json()).error.code,'round_not_editable');
    await facts(member,org,player);
    pass('restored_distinct_ui_logins_gross_net73_locked_denial_fantasy88_94_carry_receipt_and_exact_replay');
    if(!postDatabase) {
      for(const [role,p] of [['organizer',organizer],['player',member]]) {
        for(const width of [390,1280]) {
          await p.setViewportSize({width,height:900});
          await p.goto(`/leaderboard?${new URLSearchParams({tournament:fixture.tid,round:fixture.rounds[0].id,scope:'round',metric:'net'})}`);
          const row=p.locator('.leaderboard-row-link').filter({hasText:fixture.names[0]});
          await expect(row.locator('.leaderboard-score span')).toHaveText('73 netto');
          assert(await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
          await p.screenshot({path:path.join(evidenceDirectory,`restored-${role}-${width}.png`),fullPage:true});
        }
      }
      pass('restored_organizer_player_mobile_desktop_exact_results_and_no_horizontal_overflow');
      await member.goto('/profile');
      await member.evaluate(endpoint=>{
        window.recoveryStreamEvents=[]; window.recoveryStream=new EventSource(endpoint,{withCredentials:true});
        for(const name of ['open','error'])window.recoveryStream.addEventListener(name,()=>window.recoveryStreamEvents.push(name));
      },livePath);
      await expect.poll(()=>member.evaluate(()=>window.recoveryStreamEvents.filter(name=>name==='open').length),{timeout:30000}).toBe(1);
      outage=true;
      fs.writeFileSync(path.join(control.directory,'browser-ready.json'),JSON.stringify({stream_open:true}),{mode:0o600});
      await expect.poll(()=>fs.existsSync(path.join(control.directory,'restart-complete.json')),{timeout:60000}).toBe(true);
      await expect.poll(()=>member.evaluate(()=>window.recoveryStreamEvents.filter(name=>name==='open').length),{timeout:45000}).toBeGreaterThan(1);
      outage=false;
      await facts(member,org,player);
      const authoritative=await member.request.get('/api/ready'); assert.equal(authoritative.status(),200);
      await member.evaluate(()=>window.recoveryStream.close());
      pass('same_authenticated_native_stream_reopens_after_api_web_restart_and_fresh_authoritative_reads_match');
    } else pass('fresh_restored_logins_and_authoritative_reads_after_database_restart');
    await Promise.all(pending);assert.deepEqual(errors,[]);
    pass('no_unexpected_console_network_or_http_body_failures');
    fs.writeFileSync(path.join(evidenceDirectory,postDatabase?'browser-post-database.json':'browser.json'),JSON.stringify({browser:await browser.version(),checks,images:control.images,certificate_exception:'localhost disposable contexts only',native_stream:postDatabase?'not exercised in this fresh-login mode':'explicit same-origin EventSource on profile page; existing authenticated context across API/web restart'},null,2)+'\n');
  }finally{await Promise.all(contexts.map(context=>context.close()));await browser.close();}
}
main().catch(error=>{
  fs.writeFileSync(path.join(control.directory,'private-browser-failure.log'),String(error.stack)+'\n'+JSON.stringify(errors),{mode:0o600});
  console.error(JSON.stringify({failed:error.name,detail:'private diagnostics retained'}));process.exitCode=1;
});
