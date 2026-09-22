import { createServer, type ServerResponse } from 'node:http'
import type { Page } from '@playwright/test'
import { tournament, round as draft, session, completion } from '../src/features/tournaments/lifecycle/__tests__/fixtures'
import type { ScoringScorecard } from '../src/api/scorecards'

export const round = { ...draft, status: 'open' as const }
export const owner = { type: 'player' as const, id: '00000000-0000-0000-0000-000000000004' }
export const scoreUrl = `/score?tournament=${tournament.id}&round=${round.id}&owner_type=player&owner=${owner.id}&hole=8&view=hole`
export function scorecard(): ScoringScorecard {
  return { projection: 'scoring', round_id: round.id, owner, number_of_holes: 18,
    gross_total: 0, net_total: 0, playing_handicap: 0, holes_scored: 0,
    complete: false, confirmed: false, confirmed_at: null, confirmed_by: null,
    holes: Array.from({ length: 18 }, (_, i) => ({
      hole_id: `00000000-0000-0000-0001-${String(i + 1).padStart(12, '0')}`,
      hole_number: i + 1, par: 4, stroke_index: i + 1, handicap_strokes: 0, net_strokes: null, score: null,
    })) }
}

export async function liveServer() {
  const streams = new Set<ServerResponse>()
  let stopped = false
  let connections = 0
  const server = createServer((req, res) => {
    res.setHeader('Access-Control-Allow-Origin', req.headers.origin ?? 'http://127.0.0.1:5173')
    res.setHeader('Access-Control-Allow-Credentials', 'true')
    connections += 1
    if (stopped) { res.writeHead(204); res.end(); return }
    res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache' })
    res.write('retry: 100\n\n')
    streams.add(res)
    req.on('close', () => streams.delete(res))
  })
  await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve))
  const address = server.address()
  if (!address || typeof address === 'string') throw new Error('Missing live server port')
  return {
    url: `http://127.0.0.1:${address.port}/live`,
    get connections() { return connections },
    emit(type: string) { for (const res of streams) res.write(`event: ${type}\ndata: {}\n\n`) },
    stop() { stopped = true; for (const res of streams) res.end() },
    resume() { stopped = false },
    async close() {
      for (const res of streams) res.end()
      server.closeAllConnections()
      await new Promise<void>((resolve, reject) => server.close(error => error ? reject(error) : resolve()))
    },
  }
}

export async function mockWorkspace(page: Page, streamUrl: string) {
  await page.addInitScript(({ url }) => {
    const NativeEventSource = window.EventSource
    window.EventSource = class extends NativeEventSource {
      constructor(_url: string | URL, init?: EventSourceInit) { super(url, init) }
    }
  }, { url: streamUrl })
  const state = { expired: false, empty: false, fail: false, reads: 0, pending: null as Promise<void> | null,
    readOnly: false, restricted: false, denied: false, saves: 0 }
  await page.route(url => url.pathname.startsWith('/api/'), async route => {
    const path = new URL(route.request().url()).pathname
    if (path === '/api/auth/session') {
      return route.fulfill(state.expired
        ? { status: 401, json: { error: { code: 'unauthorized', message: 'Logg inn på nytt' } } }
        : { json: { ...session, expires_at: '2099-01-01T00:00:00Z' } })
    }
    if (path === '/api/tournaments') return route.fulfill({ json: [tournament] })
    if (path === '/api/me/tournaments') return route.fulfill({ json: [] })
    if (path === `/api/tournaments/${tournament.id}/rounds`) return route.fulfill({ json: [{ ...round, status: state.readOnly ? 'locked' : 'open' }] })
    if (path.endsWith('/completion-validation')) {
      state.reads += 1
      if (state.pending) await state.pending
      if (state.denied) return route.fulfill({ status: 403, json: { error: { code: 'forbidden', message: 'Du har ikke tilgang' } } })
      if (state.fail) return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Kunne ikke oppdatere scorekortet' } } })
      const progress = completion()
      if (state.restricted) return route.fulfill({ json: { ...progress, status: 'locked', visibility: { mode: 'front_nine' },
        ready_to_complete: null, ready_to_lock: null, issues: [], owners: progress.owners.map(item => ({ ...item,
          required_holes: 9, holes_scored: 0, complete: null, confirmed: null })) } })
      return route.fulfill({ json: { ...progress, status: state.readOnly ? 'locked' : 'open', ready_to_complete: false,
        owners: state.empty ? [] : progress.owners.map(item => ({ ...item, holes_scored: 0, complete: false, confirmed: false })),
        issues: [{ code: state.empty ? 'no_required_owners' : 'incomplete_scorecards', message: 'Ikke ferdig' },
          ...(state.empty ? [] : [{ code: 'unconfirmed_scorecards', message: 'Ikke bekreftet' }]),
          ...(state.readOnly ? [{ code: 'round_not_open', message: 'Låst' }] : []), ...progress.issues] } })
    }
    if (path.endsWith('/score-access')) return route.fulfill({ json: { round_id: round.id, writable_owners: state.readOnly || state.denied ? [] : [owner] } })
    if (path.endsWith('/scoring')) return route.fulfill({ json: scorecard() })
    if (path.includes('/scorecards/')) {
      const card = scorecard()
      return route.fulfill({ json: { projection: 'read', round_id: round.id, owner, number_of_holes: 18,
        visible_hole_count: state.restricted ? 9 : 18, holes: card.holes.slice(0, state.restricted ? 9 : 18),
        playing_handicap: 0, gross_total: 0, net_total: 0, holes_scored: 0,
        complete: state.restricted ? null : false, confirmed: state.restricted ? null : false,
        confirmed_at: null, visibility: { mode: state.restricted ? 'front_nine' : 'full' } } })
    }
    if (path.endsWith('/scores') || path.endsWith('/scores/conditional')) {
      state.saves += 1
      return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'Lagring utilgjengelig' } } })
    }
    throw new Error(`Unexpected browser request: ${path}`)
  })
  return state
}
