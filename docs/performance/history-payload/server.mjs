import { createServer } from 'node:http'
import { readFileSync } from 'node:fs'
import { resolve, extname } from 'node:path'
import { gzipSync } from 'node:zlib'
import { dataFor, tournamentId } from './fixtures.mjs'
export async function startFixture(root, probe) {
  let current
  const sockets = new Set()
  const server = createServer((req, res) => {
    const state = current, path = new URL(req.url, 'http://localhost').pathname
    if (path.endsWith('/live')) {
      state.streams.add(res)
      const open = () => { res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-store' }); res.write(': connected\n\n') }
      state.openers.push(open); req.on('close', () => state.streams.delete(res)); return
    }
    let body, type = 'application/json'
    if (path === '/probe.js') { body = probe; type = 'text/javascript' }
    else if (path === '/probe') { body = '<html><body>Isolated decoder probe</body></html>'; type = 'text/html' }
    else if (path.startsWith('/api/')) {
      let value
      if (path === '/api/auth/session') value = state.data.session
      else if (path === '/api/me/tournaments') value = []
      else if (path === `/api/tournaments/${tournamentId}/rounds`) value = state.data.rounds
      else if (path === `/api/tournaments/${tournamentId}/match-table`) value = state.data.table
      else if (/^\/api\/rounds\/[^/]+\/match-play\/matches$/.test(path)) value = state.data.listings.get(path.split('/')[3])
      else { state.errors.push(`Unexpected ${path}`); res.writeHead(404).end(); return }
      body = JSON.stringify(value)
    } else {
      const file = path.startsWith('/assets/') ? resolve(root, 'frontend/dist', path.slice(1)) : resolve(root, 'frontend/dist/index.html')
      if (!file.startsWith(resolve(root, 'frontend/dist') + '/')) { res.writeHead(400).end(); return }
      body = readFileSync(file); type = ({ '.js': 'text/javascript', '.css': 'text/css' })[extname(file)] ?? 'text/html'
    }
    const bytes = Buffer.from(body), encoded = gzipSync(bytes)
    const row = { path, phase: state.phase, epoch: state.epoch, rawBytes: bytes.length, gzipBytes: encoded.length }
    state.requests.push(row)
    res.on('finish', () => { row.finished = true })
    res.on('close', () => { row.aborted = !res.writableFinished })
    res.writeHead(200, { 'Content-Type': type, 'Cache-Control': 'no-store', 'Content-Encoding': 'gzip', 'Content-Length': encoded.length }); res.end(encoded)
  })
  server.on('connection', socket => { sockets.add(socket); socket.on('close', () => sockets.delete(socket)) })
  await new Promise(done => server.listen(0, '127.0.0.1', done))
  return {
    base: `http://127.0.0.1:${server.address().port}`,
    begin(scenario) { current = { data: dataFor(scenario), scenario, epoch: 0, phase: 'initial', requests: [], errors: [], streams: new Set(), openers: [] }; return current },
    open() { current.phase = 'open'; current.epoch = 1; current.data = dataFor(current.scenario, 1); for (const open of current.openers) open() },
    refresh() { current.phase = 'refresh'; current.epoch = 2; current.data = dataFor(current.scenario, 2); for (const stream of current.streams) stream.write('event: match\ndata: {}\n\n') },
    async close() { for (const socket of sockets) socket.destroy(); await new Promise(done => server.close(done)) },
  }
}
