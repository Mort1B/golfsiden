import { test, expect, type Page } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeTournamentList, decodeTournamentRounds } from '../src/api/tournaments/decoders'
test.skip(process.env.GOLF_TOURNAMENT_DETAILS_BROWSER !== '1', 'Requires fresh task-owned PostgreSQL seed and GOLF_TOURNAMENT_DETAILS_BROWSER=1.')
async function layout(page: Page, state: string) {
  const form=page.getByRole('form',{name:'Turneringsopplysninger'})
  for(const width of [320,390,1280]) {
    await page.setViewportSize({width,height:900})
    await form.scrollIntoViewIfNeeded()
    expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
    for(const button of await form.locator('button:visible').all()) expect((await button.boundingBox())?.height).toBeGreaterThanOrEqual(44)
    await form.screenshot({path:`/tmp/golf-tournament-edit-20260929/${state}-${width}.png`})
  }
}
test('draft administrator edits persist, protect round dates, handle stale/network saves and lock at start',async({page})=>{
  const errors:string[]=[];const unexpected:string[]=[]
  await page.goto('/login');await page.getByLabel('Brukernavn',{exact:true}).fill('anders');await page.getByLabel('Passord',{exact:true}).fill('golf-dev-2026')
  await page.getByRole('button',{name:'Logg inn',exact:true}).click();await expect(page).not.toHaveURL(/\/login/)
  let failuresExpected=false
  page.on('pageerror',e=>errors.push(e.message))
  page.on('console',m=>{if(m.type()==='error' && !(failuresExpected && /\/(details|rounds)$/.test(m.location().url) && /409|503|ERR_FAILED/.test(m.text()))) errors.push(m.text())})
  page.on('response',r=>{if(r.status()>=400 && !(failuresExpected && [409,503].includes(r.status()) && /\/(details|rounds)$/.test(new URL(r.url()).pathname)))unexpected.push(`${r.status()} ${new URL(r.url()).pathname}`)})
  const auth=decodeAuthSession(await(await page.request.get('/api/auth/session')).json())
  const startDate=new Date(Date.now()+7*86400000).toISOString().slice(0,10), endDate=new Date(Date.now()+8*86400000).toISOString().slice(0,10)
  const created=await page.request.post('/api/tournaments',{headers:{'x-csrf-token':auth.csrf_token},data:{request_id:crypto.randomUUID(),tournament:{name:'Test av turneringsopplysninger',description:'',start_date:startDate,end_date:endDate,counted_rounds:2,mandatory_round_number:null},rounds:[{round_number:1,name:'Første runde',round_date:startDate,scoring_format:'individual_stroke_play'},{round_number:2,name:'Andre runde',round_date:endDate,scoring_format:'individual_stroke_play'}]}})
  expect(created.status()).toBe(201)
  const receipt=await created.json()
  const trip=decodeTournamentList(await(await page.request.get('/api/tournaments')).json()).find(t=>t.id===receipt.tournament_id)!
  expect(trip.status).toBe('draft')
  const rounds=decodeTournamentRounds(await(await page.request.get(`/api/tournaments/${trip.id}/rounds`)).json(),trip.id)
  const path=`/api/tournaments/${trip.id}/details`
  await page.goto(`/manage/tournaments/${trip.id}#settings`)
  const form=page.getByRole('form',{name:'Turneringsopplysninger'})
  await expect(form.getByLabel('Turneringsnavn')).toBeEnabled()
  let release=()=>{};const held=new Promise<void>(resolve=>{release=resolve})
  await page.route(`**/api/tournaments/${trip.id}/rounds`,async route=>{await held;await route.continue()})
  await page.reload();await expect(form.getByText('Henter oppdatert turnering og rundeplan …')).toBeVisible()
  await expect(form.getByLabel('Turneringsnavn')).toBeDisabled();await layout(page,'loading')
  release();await expect(form.getByLabel('Turneringsnavn')).toBeEnabled();await page.unroute(`**/api/tournaments/${trip.id}/rounds`)
  failuresExpected=true
  await page.route(`**/api/tournaments/${trip.id}/rounds`,route=>route.fulfill({status:503,json:{error:{code:'unavailable',message:'Injected read failure'}}}))
  await page.reload();await expect(form.getByText('Kunne ikke hente rundeplanen.')).toBeVisible()
  await expect(form.getByLabel('Turneringsnavn')).toBeDisabled();await layout(page,'read-error')
  await page.unroute(`**/api/tournaments/${trip.id}/rounds`);await form.getByRole('button',{name:'Prøv igjen',exact:true}).click()
  await expect(form.getByLabel('Turneringsnavn')).toBeEnabled();failuresExpected=false
  await page.route(`**/api/tournaments/${trip.id}/rounds`,route=>route.fulfill({json:[]}))
  await page.reload();await expect(form.getByLabel('Turneringsnavn')).toBeEnabled();await layout(page,'empty-rounds')
  await page.unroute(`**/api/tournaments/${trip.id}/rounds`);await page.reload()
  await expect(form.getByLabel('Turneringsnavn')).toBeEnabled();await layout(page,'populated')
  await form.getByLabel('Turneringsnavn').fill('');await expect(form.getByRole('button',{name:'Lagre turneringsopplysninger'})).toBeDisabled();await layout(page,'empty-name')
  const name='Golfturen med venner – oppdatert navn og praktiske opplysninger'
  await form.getByLabel('Turneringsnavn').fill(name)
  await form.getByLabel('Beskrivelse (valgfritt)').fill('Ta med drikke og klær til skiftende vær. '.repeat(25))
  await form.getByLabel('Startdato',{exact:true}).fill(trip.end_date)
  await expect(form.getByText(/Perioden må inneholde/)).toBeVisible();await expect(form.getByRole('button',{name:'Lagre turneringsopplysninger'})).toBeDisabled()
  await form.getByLabel('Startdato',{exact:true}).fill(trip.start_date);await layout(page,'long-content')
  await form.getByRole('button',{name:'Lagre turneringsopplysninger'}).click()
  await expect(form.getByText('Turneringsopplysningene er lagret og kontrollert.')).toBeVisible()
  await expect(page.getByRole('heading',{name,exact:true})).toBeVisible()
  expect(decodeTournamentRounds(await(await page.request.get(`/api/tournaments/${trip.id}/rounds`)).json(),trip.id)).toEqual(rounds)
  await page.reload();await expect(form.getByLabel('Turneringsnavn')).toHaveValue(name)
  // Commit a competing real edit immediately before forwarding the form's old version.
  failuresExpected=true
  await form.getByLabel('Turneringsnavn').fill('Lokalt utkast beholdes')
  await page.route(`**${path}`,async route=>{
    const current=await(await page.request.get(`/api/tournaments/${trip.id}`)).json()
    const r=await page.request.patch(path,{headers:{'x-csrf-token':auth.csrf_token},data:{name:'Endret av annen administrator',description:current.description,start_date:current.start_date,end_date:current.end_date,expected_tournament_updated_at:current.updated_at}})
    expect(r.status()).toBe(200);await route.continue()
  })
  await form.getByRole('button',{name:'Lagre turneringsopplysninger'}).click()
  await expect(form.getByRole('button',{name:'Forkast utkast og hent siste'})).toBeVisible()
  await page.unroute(`**${path}`)
  await expect(form.getByLabel('Turneringsnavn')).toHaveValue('Lokalt utkast beholdes');await layout(page,'conflict')
  await form.getByRole('button',{name:'Forkast utkast og hent siste'}).click()
  await expect(form.getByLabel('Turneringsnavn')).toHaveValue('Endret av annen administrator')
  await page.route(`**${path}`,route=>route.abort('failed'))
  await form.getByLabel('Turneringsnavn').fill('Etter nytt forsøk')
  await form.getByRole('button',{name:'Lagre turneringsopplysninger'}).click()
  await expect(form.getByText(/Kunne ikke lagre eller kontrollere/)).toBeVisible();await layout(page,'network-error')
  await page.unroute(`**${path}`)
  await form.getByRole('button',{name:'Prøv lagring igjen'}).click();await expect(form.getByText('Turneringsopplysningene er lagret og kontrollert.')).toBeVisible()
  failuresExpected=false
  await page.goto('/tournaments');await expect(page.locator(`a[href="/tournaments/${trip.id}"]`).getByRole('heading',{name:'Etter nytt forsøk',exact:true})).toBeVisible()
  const current=await(await page.request.get(`/api/tournaments/${trip.id}`)).json()
  const start=await page.request.post(`/api/tournaments/${trip.id}/start`,{headers:{'x-csrf-token':auth.csrf_token},data:{expected_tournament_updated_at:current.updated_at}});expect(start.status()).toBe(200)
  await page.goto(`/manage/tournaments/${trip.id}#settings`)
  await expect(page.getByText(/Navn, beskrivelse og datoer kan bare endres/)).toBeVisible();await expect(form).toHaveCount(0)
  expect(errors).toEqual([]);expect(unexpected).toEqual([])
})
