import { createServer } from 'node:http'
import { readFileSync } from 'node:fs'
import { resolve, extname } from 'node:path'
import { gzipSync } from 'node:zlib'
import { fixture, tournamentId, id } from '../fixtures.mjs'

export async function startFixture(root) {
  const sockets = new Set()
  let state
  const server = createServer((req, res) => {
    const sample = state, path = new URL(req.url, 'http://localhost').pathname
    const row = { id: sample.requests.length + 1, path, startMs: Date.now() - sample.origin, epoch: sample.epoch, account: sample.account, phase: sample.phase }
    sample.requests.push(row)
    res.on('finish', () => { row.finishMs = Date.now() - sample.origin })
    res.on('close', () => { row.closeMs = Date.now() - sample.origin; row.aborted = !res.writableFinished })
    if (path.endsWith('/live')) {
      sample.streams.add(res)
      res.on('close', () => sample.streams.delete(res))
      const open = () => {
        if (res.destroyed || res.headersSent) return
        res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-store' })
        res.write(': connected\nretry: 200\n\n')
        row.openMs = Date.now() - sample.origin
      }
      sample.openers.push(open)
      if (sample.allowStream) open()
      return
    }
    let body, type, cache = 'no-store', status = 200
    if (path.startsWith('/api/')) {
      type = 'application/json'
      const data = fixture(24, 3)
      data.session.user_id = id(sample.account === 1 ? 4 : 5)
      for (const listing of data.listings.values()) for (const card of listing.matches) {
        for (const player of card.opponents) player.display_name += ` [epoch ${sample.epoch}]`
      }
      for (const player of data.table.entries) player.display_name += ` [epoch ${sample.epoch}]`
      let value
      if (path === '/api/auth/session') value = data.session
      else if (path === '/api/tournaments') value = [{ id: tournamentId, name: 'Syntetisk matchturnering', description: '', start_date: '2026-09-17', end_date: '2026-09-19', number_of_rounds: 3, counted_rounds: null, mandatory_round_id: null, status: 'active', scoring_mode: 'individual', tie_break_policy: 'shared_positions', created_at: '2026-09-17T10:00:00Z', updated_at: '2026-09-17T10:00:00Z' }]
      else if (path === '/api/me/tournaments') value = []
      else if (path === `/api/tournaments/${tournamentId}/rounds`) value = data.rounds
      else if (path === `/api/tournaments/${tournamentId}/match-table`) value = data.table
      else if (/^\/api\/rounds\/[^/]+\/match-play\/matches$/.test(path)) value = data.listings.get(path.split('/')[3])
      else { sample.errors.push(`Unexpected ${path}`); res.writeHead(404).end(); return }
      if (sample.denyPath === path) { status = sample.denyStatus; value = { error: { code: 'denied', message: 'Synthetic access denied' } } }
      row.status = status
      body = Buffer.from(JSON.stringify(value))
    } else {
      const asset = path.startsWith('/assets/')
      const file = asset ? resolve(root, 'frontend/dist', path.slice(1)) : resolve(root, 'frontend/dist/index.html')
      if (!file.startsWith(resolve(root, 'frontend/dist') + '/')) { res.writeHead(400).end(); return }
      body = readFileSync(file)
      type = ({ '.js': 'text/javascript', '.css': 'text/css' })[extname(file)] ?? 'text/html'
      if (asset) cache = 'public, max-age=31536000, immutable'
    }
    const encoded = gzipSync(body)
    row.bodyBytes = encoded.length
    const send = () => {
      row.releaseMs = Date.now() - sample.origin
      if (res.destroyed) return
      res.writeHead(status, { 'Content-Type': type, 'Cache-Control': cache, 'Content-Encoding': 'gzip', 'Content-Length': encoded.length })
      res.end(encoded)
    }
    if (sample.holdKinds.some(kind => path.endsWith('/' + kind))) {
      row.held = true; sample.held.push(send)
    } else send()
  })
  server.on('connection', socket => { sockets.add(socket); socket.on('close', () => sockets.delete(socket)) })
  await new Promise(done => server.listen(0, '127.0.0.1', done))
  return {
    base: `http://127.0.0.1:${server.address().port}`,
    begin() {
      state = { origin: Date.now(), epoch: 0, account: 1, phase: 'initial',
        allowStream: false, holdKinds: [], denyPath: '', denyStatus: 403,
        requests: [], errors: [], streams: new Set(), openers: [], held: [] }
      return state
    },
    open() { state.allowStream = true; for (const open of state.openers) open() },
    release() { for (const send of state.held.splice(0)) send() },
    visibility() { for (const stream of state.streams) stream.write('event: visibility\ndata: {}\n\n') },
    match() { for (const stream of state.streams) stream.write('event: match\ndata: {}\n\n') },
    cut() { state.allowStream = false; for (const stream of state.streams) stream.end() },
    endSample() { for (const stream of state.streams) stream.end() },
    async close() { for (const socket of sockets) socket.destroy(); await new Promise(done => server.close(done)) },
  }
}
