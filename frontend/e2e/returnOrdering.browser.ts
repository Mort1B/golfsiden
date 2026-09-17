import { test, expect, type CDPSession } from '@playwright/test'
import { liveServer, mockWorkspace, scoreUrl, scorecard, round, owner } from './returnLoadingSupport'

test.skip(process.env.GOLF_RETURN_ORDERING_REPRO !== '1', 'Opt-in regression reproducer: the pending-return defect is tracked in docs/PLANS.md.')

test('a frozen return must refresh authority after an unfinished earlier return', async ({ page, context }, testInfo) => {
  const live = await liveServer()
  const trace: string[] = []
  let release: () => void = () => undefined
  let held = false, started = 0, authReads = 0
  const pending = new Promise<void>(resolve => { release = resolve })
  const handlers: Promise<void>[] = []
  let cdp: CDPSession | undefined
  try {
    const state = await mockWorkspace(page, live.url)
    page.on('request', request => {
      const path = new URL(request.url()).pathname
      const label = path.split('/').at(-1)
      if (label === 'session') authReads++
      if (['session', 'rounds', 'score-access', 'scoring', 'completion-validation'].includes(label ?? '')) trace.push(`request ${label} locked=${state.readOnly}`)
    })
    page.on('response', response => {
      const label = new URL(response.url()).pathname.split('/').at(-1)
      if (['session', 'rounds', 'score-access', 'scoring', 'completion-validation'].includes(label ?? '')) trace.push(`response ${label} fixtureLockedAtDelivery=${state.readOnly}`)
    })
    await page.route('**/api/rounds/*/scorecards/player/*/scoring', route => {
      const result = (async () => {
        const captured = scorecard()
        if (held) { started++; trace.push(`hold scoring response capturedBeforeLock=${!state.readOnly}`); await pending }
        await route.fulfill({ json: captured })
      })()
      handlers.push(result)
      return result
    })
    await page.goto(scoreUrl)
    await expect(page.locator('#current-hole-heading')).toHaveText('8')
    live.stop()
    await expect(page.getByText('Forbindelsen er brutt. Prøver å koble til igjen.')).toBeVisible()
    await context.setOffline(true)
    held = true
    live.resume()
    const authority = ['rounds', 'completion-validation', 'score-access'].map(endpoint =>
      page.waitForResponse(response => new URL(response.url()).pathname.endsWith(`/${endpoint}`) && response.status() === 200))
    await context.setOffline(false)
    await page.evaluate(() => window.dispatchEvent(new Event('online')))
    await expect.poll(() => started).toBeGreaterThan(0)
    const responses = await Promise.all(authority)
    for (const response of responses) {
      await response.finished()
      const endpoint = new URL(response.url()).pathname.split('/').at(-1)
      const body: unknown = await response.json()
      if (endpoint === 'rounds') expect(body).toEqual([round])
      if (endpoint === 'completion-validation') expect(body).toMatchObject({ status: 'open' })
      if (endpoint === 'score-access') expect(body).toEqual({ round_id: round.id, writable_owners: [owner] })
      trace.push(`verified completed open authority: ${endpoint}`)
    }
    await expect(page.getByText('Forbindelsen er brutt. Prøver å koble til igjen.')).toHaveCount(0)
    await expect(page.getByRole('button', { name: 'Legg til ett slag' })).toBeEnabled()
    const before = authReads
    trace.push(`freeze with held scoring, auth=${before}`)
    cdp = await context.newCDPSession(page)
    await cdp.send('Page.setWebLifecycleState', { state: 'frozen' })
    state.readOnly = true
    trace.push('authority locked while frozen')
    await cdp.send('Page.setWebLifecycleState', { state: 'active' })
    await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })))
    trace.push(`after persisted pageshow auth=${authReads}`)
    held = false; release()
    await Promise.all(handlers)
    await expect(page.getByText('Du kan se dette scorekortet, men ikke føre score for det.')).toBeVisible()
    await expect(page.getByRole('button', { name: 'Legg til ett slag' })).toHaveCount(0)
    await expect(page.locator('#current-hole-heading')).toHaveText('8')
  } finally {
    release()
    await cdp?.detach()
    console.log(JSON.stringify(trace, null, 2))
    await testInfo.attach('request-ordering', { body: JSON.stringify(trace, null, 2), contentType: 'application/json' })
    await live.close()
  }
})
