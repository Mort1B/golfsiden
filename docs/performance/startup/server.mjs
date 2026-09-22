import { createServer } from 'node:http'
import { readFileSync } from 'node:fs'
import { resolve, extname } from 'node:path'
import { gzipSync } from 'node:zlib'
import { fixture, tournamentId } from '../fixtures.mjs'

export async function startFixture(root) {
  const sockets = new Set()
  let state
  const server = createServer((req, res) => {
    const sample = state, path = new URL(req.url, 'http://localhost').pathname
    const row = { id: sample.requests.length + 1, path, startMs: Date.now() - sample.origin, epoch: sample.epoch }
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
    let body, type, cache = 'no-store'
    if (path.startsWith('/api/')) {
      type = 'application/json'
      const data = fixture(24, 3)
      for (const listing of data.listings.values()) for (const card of listing.matches) {
        for (const player of card.opponents) player.display_name += ` [epoch ${sample.epoch}]`
      }
      for (const player of data.table.entries) player.display_name += ` [epoch ${sample.epoch}]`
      let value
      if (path === '/api/auth/session') value = data.session
      else if (path === '/api/me/tournaments') value = []
      else if (path === `/api/tournaments/${tournamentId}/rounds`) value = data.rounds
      else if (path === `/api/tournaments/${tournamentId}/match-table`) value = data.table
      else if (/^\/api\/rounds\/[^/]+\/match-play\/matches$/.test(path)) value = data.listings.get(path.split('/')[3])
      else { sample.errors.push(`Unexpected ${path}`); res.writeHead(404).end(); return }
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
      res.writeHead(200, { 'Content-Type': type, 'Cache-Control': cache, 'Content-Encoding': 'gzip', 'Content-Length': encoded.length })
      res.end(encoded)
    }
    if (sample.hold && (path.endsWith('/matches') || sample.holdTable && path.endsWith('/match-table'))) {
      row.held = true; sample.held.push(send)
    } else send()
  })
  server.on('connection', socket => { sockets.add(socket); socket.on('close', () => sockets.delete(socket)) })
  await new Promise(done => server.listen(0, '127.0.0.1', done))
  return {
    base: `http://127.0.0.1:${server.address().port}`,
    begin(mode) {
      state = { origin: Date.now(), epoch: ['natural', 'reconnect'].includes(mode) ? 1 : 0,
        allowStream: ['natural', 'reconnect'].includes(mode), hold: mode === 'overlap', holdTable: false,
        requests: [], errors: [], streams: new Set(), openers: [], held: [] }
      return state
    },
    open() { state.epoch++; state.allowStream = true; state.hold = false; for (const open of state.openers) open() },
    release() { for (const send of state.held.splice(0)) send() },
    match() { state.hold = true; state.holdTable = true; for (const stream of state.streams) stream.write('event: match\ndata: {}\n\n') },
    cut() { state.allowStream = false; for (const stream of state.streams) stream.end() },
    endSample() { for (const stream of state.streams) stream.end() },
    async close() { for (const socket of sockets) socket.destroy(); await new Promise(done => server.close(done)) },
  }
}
