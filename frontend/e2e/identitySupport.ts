import { expect, type Browser, type Page } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeObject, decodeString, decodeUuid } from '../src/api/decoder'
import { decodeTournamentList } from '../src/api/tournaments/decoders'

export const password = 'identity browser test password'
export const unique = () => `a3_${Date.now()}_${Math.floor(Math.random() * 10000)}`
export function deferred<T>() {
  let resolve: (value: T) => void = () => { throw new Error('Missing resolver') }
  const promise = new Promise<T>(done => { resolve = done })
  return { promise, resolve }
}
export function observe(page: Page) {
  const errors: string[] = []
  const failures: string[] = []
  page.on('pageerror', error => errors.push(error.message))
  page.on('console', message => { if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push(message.text()) })
  page.on('requestfailed', request => { if (request.failure()?.errorText !== 'net::ERR_ABORTED' && !request.url().endsWith('/live')) failures.push(new URL(request.url()).pathname) })
  return { errors, failures }
}
export async function layout(page: Page, width: number) {
  await page.setViewportSize({ width, height: 900 })
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width)
  for (const button of await page.locator('button:visible').all()) expect((await button.boundingBox())?.height).toBeGreaterThanOrEqual(44)
}
export async function reviewCreation(page: Page, username: string) {
  await page.goto('/')
  await page.getByRole('link', { name: /Opprett turnering/ }).click()
  await page.getByLabel('Turneringsnavn', { exact: true }).fill(`Langt turneringsnavn for kontoeierskap og sikre svar ${username}`)
  await page.getByRole('button', { name: 'Neste', exact: true }).click()
  await page.getByRole('button', { name: 'Neste', exact: true }).click()
  await page.getByLabel('Visningsnavn', { exact: true }).fill('Spiller med et langt visningsnavn for kontoeierskap')
  await page.getByLabel(/^Brukernavn/).fill(username)
  await page.getByLabel(/^Passord/).fill(password)
  await page.getByLabel(/^Handicapindeks/).fill('12,4')
  await page.getByRole('button', { name: 'Neste', exact: true }).click()
  await expect(page.getByRole('heading', { name: 'Kontroller opplysningene' })).toBeVisible()
}
export async function enterLogin(page: Page) {
  await page.goBack()
  await page.getByRole('link', { name: 'Logg inn' }).click()
  await page.getByLabel('Brukernavn', { exact: true }).fill('anders')
  await page.getByLabel('Passord', { exact: true }).fill('golf-dev-2026')
}
export async function currentIsAnders(page: Page, excludedTournament?: string) {
  await expect(page).toHaveURL(/\/tournaments$/)
  await expect(page.locator('.session-bar')).toContainText('Anders')
  const actual = decodeAuthSession(await (await page.request.get('/api/auth/session')).json())
  expect(actual.username).toBe('anders')
  const trips = decodeTournamentList(await (await page.request.get('/api/tournaments')).json())
  expect(trips.some(trip => trip.id === excludedTournament)).toBe(false)
  if (excludedTournament) expect(page.url().includes(excludedTournament)).toBe(false)
}
interface InvitationFixture {
  tournament: { id: string; name: string }
  invitation: { id: string; token: string }
}
export async function invitation(browser: Browser): Promise<InvitationFixture> {
  const context = await browser.newContext({ baseURL: 'http://127.0.0.1:5173' })
  try {
    const date = new Date(Date.now() + 86_400_000).toISOString().slice(0, 10)
    const result = await context.request.post('/api/onboarding/tournaments', { data: {
      creator: { account: { username: unique(), password }, player: { display_name: 'Arrangør', handicap_index: 8 } },
      tournament: { name: `En lang invitasjonsturnering med tydelig kontoeierskap ${Date.now()}`, description: '', start_date: date, end_date: date, counted_rounds: 1, mandatory_round_number: null },
      rounds: [{ round_number: 1, name: 'Første runde', round_date: date, scoring_format: 'individual_stroke_play' }],
    } })
    expect(result.status()).toBe(201)
    const data = decodeObject(await result.json(), 'onboarding')
    const trip = decodeObject(data.tournament, 'tournament')
    const invite = decodeObject(data.invitation, 'invitation')
    return { tournament: { id: decodeUuid(trip.id, 'tournament.id'), name: decodeString(trip.name, 'tournament.name') },
      invitation: { id: decodeUuid(invite.id, 'invitation.id'), token: decodeString(invite.token, 'invitation.token') } }
  } finally { await context.close() }
}
export async function enterInvitation(page: Page, created: InvitationFixture) {
  await page.goto('/')
  // A native same-document history transition exercises the router lifetime,
  // retaining this document's in-flight request coordinator on Back.
  await page.evaluate(path => {
    window.history.pushState(null, '', path)
    window.dispatchEvent(new PopStateEvent('popstate'))
  }, `/join/${created.invitation.id}#token=${created.invitation.token}`)
  await expect(page.getByRole('heading', { name: 'Ny spiller' })).toBeVisible()
}
export async function fillRegistration(page: Page) {
  const section = page.locator('.join-section').filter({ has: page.getByRole('heading', { name: 'Ny spiller' }) })
  await section.getByLabel('Visningsnavn', { exact: true }).fill('Invitert spiller med et langt navn')
  await section.getByLabel(/^Brukernavn/).fill(unique())
  await section.getByLabel(/^Passord/).fill(password)
  await section.getByLabel(/^Handicapindeks/).fill('12,4')
  await section.getByRole('button', { name: 'Opprett konto og bli med' }).click()
}
