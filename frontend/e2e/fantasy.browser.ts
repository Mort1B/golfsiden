import { expect, test } from '@playwright/test'
import { decodeArray, decodeObject, decodeUuid } from '../src/api/decoder'
import { results } from '../src/api/fantasy/results'
import { fantasyFixture, fantasyLayouts } from './fantasySupport'
test.skip(process.env.GOLF_FANTASY_BROWSER!=='1','Requires disposable local database and GOLF_FANTASY_BROWSER=1.')
test('Fantasy selection, settlement and both boards work at mobile and desktop widths',async({page})=>{
  const errors:string[]=[],failed:string[]=[],canceledConflicts:string[]=[],responseChecks:Promise<void>[]=[],conflicts:{status:number;path:string;code:unknown}[]=[]
  page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error'&&!m.text().includes('Failed to load resource'))errors.push(m.text())})
  page.on('requestfailed',r=>{if(!r.url().includes('/live')&&!r.failure()?.errorText.includes('ERR_ABORTED'))failed.push(r.url())})
  page.on('response',r=>{if(r.status()>=400)responseChecks.push((async()=>{
    const path=new URL(r.url()).pathname
    try {
      expect(r.status()).toBe(409)
      expect(r.request().method()).toBe('GET')
      expect(path).toMatch(/^\/api\/tournaments\/[^/]+\/fantasy\/(?:rounds\/[^/]+\/)?results$/)
      const completionError=await r.finished()
      if(r.request().failure()?.errorText==='net::ERR_ABORTED') { canceledConflicts.push(path); return }
      if(completionError)throw completionError
      const envelope=decodeObject(await r.json(),'error'),error=decodeObject(envelope.error,'error.error')
      conflicts.push({status:r.status(),path,code:error.code})
    } catch(error) {
      failed.push(`HTTP ${r.status()} response: ${path}: ${error instanceof Error?error.message:String(error)}`)
    }
  })())})
  const f=await fantasyFixture(page),first=f.rounds[0],second=f.rounds[1],captain=f.players[1]
  if(!first||!second||!captain)throw new Error('Missing fixture round/player')
  await page.goto(f.url);await expect(page.getByText('Fantasy er ikke aktivert for denne turneringen.')).toBeVisible();await fantasyLayouts(page,'disabled')
  await page.getByRole('button',{name:'Aktiver Fantasy',exact:true}).click();await expect(page.getByText('Ingen Fantasy-lag er påmeldt.')).toBeVisible()
  await page.getByRole('button',{name:'Meld meg på Fantasy'}).click()
  for(const name of f.names)await page.getByRole('checkbox',{name,exact:true}).check()
  await page.getByLabel('Kaptein · doble poeng').selectOption(captain);await page.getByRole('button',{name:'Lagre firer og kaptein'}).click();await expect(page.getByText('Lagret lag · revisjon 1')).toBeVisible();await fantasyLayouts(page,'saved')
  await page.getByText('Administrer denne Fantasy-runden',{exact:true}).click()
  await page.getByLabel('Tidligere frist (din lokale tid)').fill('2090-01-01T12:00');await page.getByRole('button',{name:'Lagre valgfrist'}).click();await expect(page.getByText(/Publisert frist:/)).toBeVisible()
  await f.prepare();await f.mutate(`/api/rounds/${first.id}/open`)
  const path=`/api/rounds/${first.id}/scorecards/player/${f.players[0]}`,card=decodeObject(await(await page.request.get(`${path}/scoring`)).json(),'card')
  const holes=decodeArray(card.holes,'card.holes',(v,p)=>decodeUuid(decodeObject(v,p).hole_id,`${p}.hole_id`))
  for(const hole_id of holes)await f.mutate(`/api/rounds/${first.id}/scores`,{owner:{type:'player',id:f.players[0]},hole_id,gross_strokes:4},'PUT')
  await f.mutate(`${path}/confirm`)
  await expect(page.getByText('Låst lag',{exact:true})).toBeVisible()
  for(const player of f.players.slice(1)){await page.getByLabel('Spiller eller lag',{exact:true}).selectOption(`player:${player}`);await page.getByLabel('Begrunnelse (maks. 500 byte)').fill('Ikke fullført i lokal nettlesertest');await page.getByRole('button',{name:'Registrer ikke fullført',exact:true}).click();await expect(page.getByText(/Registrert som ikke fullført/)).toBeVisible();await expect(page.locator('.fantasy-page').getByText('Handlingen er bekreftet. Visningen er oppdatert.')).toBeVisible()}
  const total=results(await(await page.request.get(`/api/tournaments/${f.tournament.id}/fantasy/results`)).json(),f.tournament.id)
  expect(total.managers[0]?.points).toEqual({state:'provisional',total:22})
  const changedHole=holes[0];if(!changedHole)throw new Error('Missing correction hole')
  await f.mutate(`/api/rounds/${first.id}/scores`,{owner:{type:'player',id:captain},hole_id:changedHole,gross_strokes:6},'PUT')
  await page.getByLabel('Spiller eller lag',{exact:true}).selectOption(`player:${captain}`)
  await expect(page.getByText(/Avgjørelsen er foreldet/)).toBeVisible()
  await page.getByLabel('Begrunnelse (maks. 500 byte)').fill('Kontrollert registrert hull etter korreksjon')
  await expect(page.getByRole('button',{name:'Bekreft ikke fullført på nytt'})).toBeDisabled()
  await page.getByRole('checkbox',{name:/Jeg har kontrollert grunnlaget/}).check()
  await page.getByRole('button',{name:'Bekreft ikke fullført på nytt'}).click()
  await expect(page.getByText('Avgjørelsen gjelder gjeldende scoregrunnlag.')).toBeVisible()
  const corrected=results(await(await page.request.get(`/api/tournaments/${f.tournament.id}/fantasy/results`)).json(),f.tournament.id)
  expect(corrected.managers[0]?.points).toEqual({state:'provisional',total:20})
  await page.getByRole('button',{name:'Spillerpoeng',exact:true}).click();await expect(page.locator('.fantasy-standings > li')).toHaveCount(4);await page.locator('.fantasy-standings button').filter({hasText:f.names[0]}).click();await expect(page.getByRole('heading',{name:`${f.names[0]} · spillerpoeng`,exact:true})).toBeVisible();await fantasyLayouts(page,'points')
  await page.getByLabel('Vis poeng for').selectOption(first.id);await expect(page.locator('.fantasy-standings > li')).toHaveCount(4)
  await page.getByRole('button',{name:'Fantasy-lag',exact:true}).click();await page.locator('.fantasy-standings button').first().click();await expect(page.locator('.fantasy-contributions li')).toHaveCount(4);await expect(page.getByText(/kaptein ×2/).last()).toBeVisible()
  await page.getByLabel('Runde for Min firer').selectOption(second.id);await expect(page.getByText('Hvis du ikke leverer et nytt gyldig lag')).toBeVisible();await expect(page.getByText(/Dette laget kan gjenbrukes ved fristen/)).toBeVisible()
  await page.evaluate(()=>window.dispatchEvent(new Event('pageshow')));await expect(page.getByText(/Dette laget kan gjenbrukes ved fristen/)).toBeVisible()
  await Promise.all(responseChecks)
  for(const response of conflicts){expect(response.status).toBe(409);expect(response.path).toContain('/fantasy');expect(response.code).toBe('fantasy_conflict')}
  await expect(page.locator('.fantasy-page [role=alert]')).toHaveCount(0)
  const fresh=await page.request.get(`/api/tournaments/${f.tournament.id}/fantasy/results`);expect(fresh.status()).toBe(200)
  console.log('Concurrent Fantasy projection conflicts:',JSON.stringify(conflicts))
  console.log('Canceled Fantasy result reads with HTTP 409 (body not verified):',canceledConflicts.length)
  expect(errors).toEqual([]);expect(failed).toEqual([])
})

test('Fantasy exposes loading, retry and denied-access states without retaining private boards',async({page})=>{
  const f=await fantasyFixture(page)
  await f.mutate(`/api/tournaments/${f.tournament.id}/fantasy`,{enabled:true},'PUT')
  let release:()=>void=()=>{}
  const held=new Promise<void>(resolve=>{release=resolve})
  await page.route(`**/api/tournaments/${f.tournament.id}/fantasy`,async route=>{await held;await route.continue()})
  await page.goto(f.url)
  await expect(page.locator('.fantasy-page').getByText('Laster …').first()).toBeVisible()
  await fantasyLayouts(page,'loading');release()
  await expect(page.getByText('Ingen Fantasy-lag er påmeldt.')).toBeVisible()
  await page.unroute(`**/api/tournaments/${f.tournament.id}/fantasy`)
  await page.route(`**/api/tournaments/${f.tournament.id}/fantasy/results`,route=>route.fulfill({status:503,contentType:'application/json',body:JSON.stringify({error:{code:'unavailable',message:'Resultatene er midlertidig utilgjengelige'}})}))
  await page.getByRole('button',{name:'Oppdater Fantasy'}).click();await expect(page.getByText('Resultatene er midlertidig utilgjengelige')).toBeVisible();await expect(page.locator('.fantasy-standings')).toHaveCount(0);await fantasyLayouts(page,'error')
  await page.unroute(`**/api/tournaments/${f.tournament.id}/fantasy/results`)
  await page.getByRole('button',{name:'Prøv igjen',exact:true}).click();await expect(page.getByText('Ingen Fantasy-lag er påmeldt.')).toBeVisible()
  await page.route(`**/api/tournaments/${f.tournament.id}/fantasy/results`,route=>route.fulfill({status:403,contentType:'application/json',body:JSON.stringify({error:{code:'forbidden',message:'Tilgangen til Fantasy er fjernet'}})}))
  await page.getByRole('button',{name:'Oppdater Fantasy'}).click();await expect(page.getByText('Tilgangen til Fantasy er fjernet').first()).toBeVisible();await expect(page.locator('.fantasy-standings')).toHaveCount(0);await fantasyLayouts(page,'denied')
})
