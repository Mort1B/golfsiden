import { readFile } from 'node:fs/promises'
import { expect, test } from '@playwright/test'
import { decodeArray, decodeObject, decodeString, decodeUuid } from '../src/api/decoder'
import { game } from '../src/api/fantasy/selections'
import { breakdown, golfer, manager, results, roundResult } from '../src/api/fantasy/results'
import { decodeStablefordScoring } from '../src/api/stableford/decoders'
import { at, browserEvidence, releaseFixture, selectFour } from './fantasyReleaseSupport'
import { fantasyLayouts } from './fantasySupport'

test.skip(process.env.GOLF_FANTASY_BROWSER !== '1', 'Requires disposable local API and Chrome.')
test('schema-38 legacy, unlocked and expired games preserve the intended rules after upgrade',async({browser})=>{
 const path=process.env.GOLF_FANTASY_LEGACY_FIXTURES
 test.skip(!path,'Requires pre-upgrade schema-38 fixtures and their browser sessions.')
 if(!path)throw new Error('Missing upgrade fixtures')
 const fixtures=decodeArray(JSON.parse(await readFile(path,'utf8')) as unknown,'fixtures',(v,p)=>{
  const x=decodeObject(v,p);return {kind:decodeString(x.kind,`${p}.kind`),state:decodeString(x.state,`${p}.state`),tournament:decodeUuid(x.tournament,`${p}.tournament`),round:decodeUuid(x.round,`${p}.round`),base:decodeString(x.base,`${p}.base`),before:x.before}
 })
 for(const f of fixtures){
  const context=await browser.newContext({storageState:f.state,baseURL:'http://127.0.0.1:5173'}),page=await context.newPage(),evidence=browserEvidence(page)
  try{
   const version=f.kind==='unlocked'?2:1
   expect(game(await(await page.request.get(f.base)).json(),f.tournament)?.rules_version).toBe(version)
   const overall=results(await(await page.request.get(`${f.base}/results`)).json(),f.tournament)
   expect(overall.rules_version).toBe(version)
   if(f.kind==='legacy'){
    const before=results(f.before,f.tournament)
    expect(overall.golfers).toEqual(before.golfers);expect(overall.managers).toEqual(before.managers)
    const board=roundResult(await(await page.request.get(`${f.base}/rounds/${f.round}/results`)).json(),f.tournament,f.round)
    expect(board.rules_version).toBe(1)
    const player=at(board.golfers,0),entry=at(board.managers,0)
    expect(player.holes.filter(h=>h.category==='ace'||h.category==='albatross_or_better').every(h=>h.points.state==='settled'&&h.points.total===10)).toBe(true)
    expect(breakdown(await(await page.request.get(`${f.base}/results/golfers/${player.player_id}`)).json(),f.tournament,player.player_id,golfer).rules_version).toBe(1)
    expect(breakdown(await(await page.request.get(`${f.base}/results/managers/${entry.user_id}`)).json(),f.tournament,entry.user_id,manager).rules_version).toBe(1)
   }
   await page.goto(`/tournaments/${f.tournament}/fantasy`)
   const rules=page.getByText('Regler og poeng',{exact:true});await rules.focus();await page.keyboard.press('Enter')
   await expect(page.locator('.fantasy-rules')).toContainText(`Dette spillet bruker regelversjon ${version}`)
   await expect(page.locator('.fantasy-rules')).toContainText(version===1?'Faktisk hole-in-one +10':'ett registrert bruttoslag) +20')
   if(f.kind==='legacy'){
    await page.getByRole('button',{name:'Poengtavler',exact:true}).click()
    await page.locator('.fantasy-standings button').first().click()
    await expect(page.locator('.fantasy-contributions')).toBeVisible()
    await fantasyLayouts(page,'scoring-legacy-manager')
    await page.getByRole('button',{name:'Spillerpoeng',exact:true}).click()
    await page.locator('.fantasy-standings button').first().click()
    await expect(page.locator('.fantasy-holes')).toBeVisible()
   }
   await fantasyLayouts(page,`scoring-${f.kind}-rules`);await evidence()
  }finally{await context.close()}
 }
})

test('version 2 displays ace 20, non-ace albatross 5, captain totals and matching ranks',async({page})=>{
 const evidence=browserEvidence(page),f=await releaseFixture(page,['individual_stableford'],[36,36,36,36])
 await f.configure(0,[0,1,2,3],false,3);await selectFour(f,0,[0,1,2,3],0);await f.start()
 const round=at(f.rounds,0);await f.mutate(`/api/rounds/${round.id}/open`)
 for(const [index,player] of f.players.entries()){
  const path=`/api/rounds/${round.id}/stableford/scorecards/${player}`
  const card=decodeStablefordScoring(await(await page.request.get(`${path}/scoring`)).json(),round.id,player)
  for(const [i,hole] of card.holes.entries()){
   const gross=index===0?(i===0?2:i===1?1:5):index===1?(i===0?2:i===1?4:5):index===2&&i<2?4:5
   await f.mutate(`/api/rounds/${round.id}/stableford/inputs/conditional`,{request_id:crypto.randomUUID(),owner:{type:'player',id:player},hole_id:hole.hole_id,input:{type:'numeric',gross_strokes:gross},expected_score:{type:'absent'}},'PUT')
  }
  await f.mutate(`${path}/confirm`)
 }
 await f.mutate(`/api/rounds/${round.id}/complete`)
 const board=await f.board(0);expect(board.rules_version).toBe(2)
 expect(board.golfers.map(g=>g.points)).toEqual([35,14,8,5].map(total=>({state:'settled',total})))
 expect(board.golfers.map(g=>g.rank)).toEqual([1,2,3,4]);expect(board.managers[0]?.points).toEqual({state:'settled',total:97})
 const first=at(board.golfers,0)
 expect(first.holes[0]?.points).toEqual({state:'settled',total:5});expect(first.holes[1]?.points).toEqual({state:'settled',total:20})
 await page.goto(f.url);await page.getByText('Regler og poeng',{exact:true}).click()
 await expect(page.locator('.fantasy-rules')).toContainText('Dette spillet bruker regelversjon 2')
 await page.getByRole('button',{name:'Poengtavler',exact:true}).click()
 await expect(page.locator('.fantasy-standings button').first()).toContainText('97 p · avgjort')
 await page.locator('.fantasy-standings button').first().click()
 await expect(page.locator('.fantasy-contributions li').first()).toContainText('Bidrag ×2: 70 p · avgjort')
 await fantasyLayouts(page,'scoring-v2-manager')
 await page.getByRole('button',{name:'Spillerpoeng',exact:true}).click()
 await page.locator('.fantasy-standings button').first().click()
 await expect(page.locator('.fantasy-holes li').nth(0)).toContainText('5 p · avgjort')
 await expect(page.locator('.fantasy-holes li').nth(1)).toContainText('20 p · avgjort')
 await fantasyLayouts(page,'scoring-v2-golfer');await evidence()
})

test('rules remain absent during loading/error and incompatible boards recover with reads only',async({page})=>{
 const f=await releaseFixture(page,['individual_stroke_play'],[0,0,0,0]),errors:string[]=[],failed:string[]=[],httpErrors:number[]=[],writes:string[]=[]
 page.on('pageerror',error=>errors.push(error.message))
 page.on('console',message=>{if(message.type()==='error'&&!message.text().includes('Failed to load resource'))errors.push(message.text())})
 page.on('requestfailed',request=>{if(!request.url().includes('/live')&&!request.failure()?.errorText.includes('ERR_ABORTED'))failed.push(request.url())})
 page.on('response',response=>{if(response.status()>=400)httpErrors.push(response.status())})
 page.on('request',request=>{if(request.method()!=='GET')writes.push(request.method())})
 let release:()=>void=()=>{},arrived:()=>void=()=>{}
 const gate=new Promise<void>(resolve=>{release=resolve}),seen=new Promise<void>(resolve=>{arrived=resolve})
 await page.route(`**${f.base}`,async route=>{arrived();await gate;await route.fulfill({status:500,contentType:'application/json',body:JSON.stringify({error:{code:'test_error',message:'Temporary fixture failure'}})})})
 await page.goto(f.url);await seen
 await expect(page.getByText('Regler og poeng',{exact:true})).toHaveCount(0)
 await fantasyLayouts(page,'scoring-loading');release()
 await expect(page.getByText('Prøv igjen',{exact:true})).toBeVisible()
 await expect(page.getByText('Regler og poeng',{exact:true})).toHaveCount(0)
 await fantasyLayouts(page,'scoring-error')
 await page.unroute(`**${f.base}`)
 await page.route(`**${f.base}/results`,async route=>{
  const response=await route.fetch(),body=decodeObject(await response.json(),'results')
  await route.fulfill({response,json:{...body,rules_version:1}})
 })
 await page.getByText('Prøv igjen',{exact:true}).click()
 await page.getByRole('button',{name:'Poengtavler',exact:true}).click()
 await expect(page.getByText(/Poengene bruker en annen regelversjon/)).toBeVisible()
 await expect(page.locator('.fantasy-standings')).toHaveCount(0)
 await page.getByText('Regler og poeng',{exact:true}).click()
 await expect(page.locator('.fantasy-rules')).toContainText('regelversjon 2')
 await fantasyLayouts(page,'scoring-mismatch')
 await page.unroute(`**${f.base}/results`)
 const retry=page.getByRole('button',{name:'Oppdater regelversjon og poeng'})
 await retry.focus();await page.keyboard.press('Enter')
 await expect(page.getByLabel('Vis poeng for')).toBeVisible()
 await expect(page.getByText(/Poengene bruker en annen regelversjon/)).toHaveCount(0)
 await fantasyLayouts(page,'scoring-refreshed')
 expect(writes).toEqual([]);expect(errors).toEqual([]);expect(failed).toEqual([])
 expect(httpErrors.length).toBeGreaterThan(0);expect(httpErrors.every(status=>status===500)).toBe(true)
})
