import { expect, type Page } from '@playwright/test'

interface ReadEvidence {
  method: string
  path: string
  status: number
  code: unknown
  bodyReadable: boolean
  failure: string | null
}
export function classifyFantasyRead(value: ReadEvidence, conflictPaths: ReadonlySet<string>, unavailablePath: string) {
  const allowed = value.method === 'GET' && (value.status === 409 && conflictPaths.has(value.path)
    || value.status === 503 && value.path === unavailablePath)
  if (!allowed) return 'unexpected'
  if (!value.bodyReadable) return value.failure === 'net::ERR_ABORTED' || value.failure === 'Load request cancelled'
    ? 'cancelled' : 'unexpected'
  if (value.status === 409 && value.code === 'fantasy_conflict') return 'conflict'
  if (value.status === 503 && value.code === 'unavailable') return 'unavailable'
  return 'unexpected'
}

export function observeFantasyReads(page: Page, conflictPaths: ReadonlySet<string>, unavailablePath: string) {
  const checks: Promise<void>[] = [], failures: string[] = []
  const counts = { conflict: 0, unavailable: 0, cancelled: 0 }
  const conflicts = new Set<string>()
  page.on('pageerror', error => failures.push(error.message))
  page.on('console', message => { if (message.type() === 'error' && !message.text().includes('Failed to load resource')) failures.push(message.text()) })
  page.on('requestfailed', request => {
    const failure = request.failure()?.errorText
    if (request.method() === 'GET' && (failure === 'net::ERR_ABORTED' || failure === 'Load request cancelled')) return
    failures.push(`${request.method()} ${new URL(request.url()).pathname}: ${failure}`)
  })
  page.on('response', response => {
    if (response.status() < 400) return
    checks.push((async () => {
      const request = response.request(), path = new URL(response.url()).pathname
      let bodyReadable = true, code: unknown
      try {
        const body: unknown = await response.json()
        if (body && typeof body === 'object' && 'error' in body && body.error
          && typeof body.error === 'object' && 'code' in body.error) code = body.error.code
      } catch { bodyReadable = false }
      const result = classifyFantasyRead({ method: request.method(), path, status: response.status(), code,
        bodyReadable, failure: request.failure()?.errorText ?? null }, conflictPaths, unavailablePath)
      if (result === 'unexpected') failures.push(`${request.method()} ${path}: ${response.status()} (${bodyReadable ? String(code) : 'unread body without expected cancellation'})`)
      else { counts[result]++; if (result === 'conflict') conflicts.add(path) }
    })().catch(error => { failures.push(String(error)) }))
  })
  return async () => {
    // New responses can arrive while earlier body reads finish.
    let checked = 0
    while (checked < checks.length) { const end = checks.length; await Promise.all(checks.slice(checked, end)); checked = end }
    expect(failures).toEqual([])
    return { ...counts, conflicts }
  }
}
