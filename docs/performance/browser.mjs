// Run after npm --prefix frontend run build. Uses production assets + HTTP fixtures.
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { resolve, extname } from 'node:path'
import { readFileSync, writeFileSync, mkdirSync, readdirSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { createServer } from 'node:http'
import { gzipSync } from 'node:zlib'
import { cpus, totalmem } from 'node:os'
import { execFileSync } from 'node:child_process'
import { fixture, tournamentId, playerId } from './fixtures.mjs'
const root = fileURLToPath(new URL('../../', import.meta.url))
const require = createRequire(resolve(root, 'frontend/package.json'))
const { chromium } = require('playwright')
const out = process.argv[2] ?? '/tmp/golf-performance'
const streamDelayMs = Number(process.env.BASELINE_STREAM_DELAY_MS ?? 0)
mkdirSync(out, { recursive: true })
let current, signedIn = true
const unexpected = []
const streams = new Set()
const server = createServer((req, res) => {
  const path = new URL(req.url, 'http://localhost').pathname
  if (path.endsWith('/live')) {
    const connect = () => { res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-store' }); res.write(': connected\n\n') }
    const timer = streamDelayMs ? setTimeout(connect, streamDelayMs) : null
    if (!timer) connect()
    streams.add(res); req.on('close', () => { clearTimeout(timer); streams.delete(res) }); return
  }
  let body, type, cache = 'no-store', status = 200
  if (path.startsWith('/api/')) {
    type = 'application/json'
    let data
    if (path === '/api/auth/session') data = signedIn ? current.session : null
    else if (path === '/api/me/tournaments') data = []
    else if (path === `/api/tournaments/${tournamentId}/rounds`) data = current.rounds
    else if (path === `/api/tournaments/${tournamentId}/match-table`) data = current.table
    else if (path.match(/^\/api\/rounds\/[^/]+\/match-play\/matches$/)) data = current.listings.get(path.split('/')[3])
    else { unexpected.push(path); status = 404; data = { error: { code: 'fixture_missing', message: path } } }
    body = Buffer.from(JSON.stringify(data))
  } else {
    const asset = path.startsWith('/assets/')
    const file = asset ? resolve(root, 'frontend/dist', path.slice(1)) : resolve(root, 'frontend/dist/index.html')
    if (!file.startsWith(resolve(root, 'frontend/dist') + '/')) { res.writeHead(400).end(); return }
    body = readFileSync(file)
    type = ({ '.js': 'text/javascript', '.css': 'text/css' })[extname(file)] ?? 'text/html'
    if (asset) cache = 'public, max-age=31536000, immutable'
  }
  const encoded = gzipSync(body)
  res.writeHead(status, { 'Content-Type': type, 'Cache-Control': cache, 'Content-Encoding': 'gzip', 'Content-Length': encoded.length })
  res.end(encoded)
})
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
const base = `http://127.0.0.1:${server.address().port}`
const browser = await chromium.launch({ channel: 'chrome', headless: true })
const samples = [], errors = []
const cases = [
  { name: 'login', width: 390, matches: 0, rounds: 0, repeats: 5 },
  { name: 'populated', width: 390, matches: 12, rounds: 1, repeats: 5 },
  { name: 'long', width: 390, matches: 24, rounds: 3, repeats: 5 },
  { name: 'history', width: 390, matches: 24, rounds: 3, repeats: 5 },
  { name: 'stress', width: 390, matches: 100, rounds: 3, repeats: 5 },
  { name: 'long', width: 320, matches: 24, rounds: 3, repeats: 3 },
  { name: 'long', width: 1280, matches: 24, rounds: 3, repeats: 3 },
].filter(scenario => !process.env.BASELINE_CASE || `${scenario.name}-${scenario.width}` === process.env.BASELINE_CASE)
if (process.env.BASELINE_REPEATS) for (const scenario of cases) scenario.repeats = Number(process.env.BASELINE_REPEATS)
try {
  for (const scenario of cases) {
    current = fixture(scenario.matches, scenario.rounds, scenario.name !== 'populated')
    signedIn = scenario.name !== 'login'
    const path = signedIn ? `/tournaments/${tournamentId}/match-results${scenario.name === 'history' ? `?player=${playerId}` : ''}` : '/login'
    for (let repeat = 0; repeat < scenario.repeats; repeat++) {
      const context = await browser.newContext({ viewport: { width: scenario.width, height: scenario.width === 320 ? 600 : scenario.width === 390 ? 844 : 900 }, isMobile: scenario.width < 600, deviceScaleFactor: scenario.width < 600 ? 3 : 1 })
      const page = await context.newPage()
      page.setDefaultTimeout(30000)
      page.on('pageerror', error => errors.push(error.message))
      page.on('console', message => { if (message.type() === 'error') errors.push(message.text()) })
      page.on('response', response => { if (response.status() >= 400) errors.push(`HTTP ${response.status()} ${new URL(response.url()).pathname}`) })
      page.on('requestfailed', request => { if (request.failure()?.errorText !== 'net::ERR_ABORTED') errors.push(request.failure()?.errorText) })
      await page.addInitScript(() => {
        window.baseline = { longTasks: [], lcp: [] }
        new PerformanceObserver(list => window.baseline.longTasks.push(...list.getEntries().map(e => ({ start: e.startTime, duration: e.duration })))).observe({ type: 'longtask', buffered: true })
        new PerformanceObserver(list => window.baseline.lcp.push(...list.getEntries().map(e => e.startTime))).observe({ type: 'largest-contentful-paint', buffered: true })
      })
      const cdp = await context.newCDPSession(page)
      await cdp.send('Network.enable')
      await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 100, downloadThroughput: 200000, uploadThroughput: 93750 })
      await cdp.send('Emulation.setCPUThrottlingRate', { rate: 4 })
      for (const cache of ['cold', 'warm']) {
        const requests = [], pending = new Set()
        const observe = req => { requests.push(new URL(req.url()).pathname); if (req.resourceType() !== 'eventsource') pending.add(req) }
        const finished = req => pending.delete(req)
        page.on('request', observe)
        page.on('requestfinished', finished)
        page.on('requestfailed', finished)
        await page.goto(base + path, { waitUntil: 'domcontentloaded' })
        const expectedCards = scenario.name === 'history' ? scenario.rounds : scenario.matches * scenario.rounds
        await page.waitForFunction(({ signedIn, expectedCards }) => signedIn
          ? document.querySelectorAll('.match-list article').length === expectedCards && document.querySelectorAll('.match-table li').length > 0
          : !!document.querySelector('input[autocomplete="username"]'), { signedIn, expectedCards })
        // Wait for a rendered frame. Includes the synthetic network, decode and render path.
        const readyMs = await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve(performance.now())))))
        await page.waitForTimeout(700) // fixed observation window for stream-open invalidation
        const measurements = await page.evaluate(() => ({
          fcpMs: performance.getEntriesByName('first-contentful-paint')[0]?.startTime ?? null,
          lcpMs: window.baseline.lcp.at(-1) ?? null,
          longTaskCount: window.baseline.longTasks.length,
          longTaskMs: window.baseline.longTasks.reduce((sum, t) => sum + t.duration, 0),
          tbtProxyMs: window.baseline.longTasks.reduce((sum, t) => sum + Math.max(0, t.duration - 50), 0),
          domNodes: document.querySelectorAll('*').length,
          overflow: document.documentElement.scrollWidth > innerWidth,
          finalCards: document.querySelectorAll('.match-list article').length,
          finalRows: document.querySelectorAll('.match-table li').length,
          resources: performance.getEntriesByType('resource').map(r => ({ path: new URL(r.name).pathname, transferBytes: r.transferSize, encodedBytes: r.encodedBodySize, decodedBytes: r.decodedBodySize, durationMs: r.duration })),
        }))
        page.off('request', observe)
        page.off('requestfinished', finished)
        page.off('requestfailed', finished)
        const pendingPaths = [...pending].map(req => new URL(req.url()).pathname)
        if (pendingPaths.length) errors.push(`Unfinished requests ${JSON.stringify(pendingPaths)}`)
        const expectedRows = !signedIn ? 0 : scenario.name === 'history' ? 1 : scenario.matches * 2
        if (measurements.finalCards !== expectedCards || measurements.finalRows !== expectedRows) errors.push(`Unsettled content ${scenario.name} ${scenario.width}`)
        if (measurements.overflow) errors.push(`Overflow ${scenario.name} ${scenario.width}`)
        samples.push({ ...scenario, repeat: repeat + 1, cache, readyMs, ...measurements, requests, pendingPaths })
        console.log(`${scenario.name} ${scenario.width} #${repeat + 1} ${cache}: ${readyMs.toFixed(0)} ms; API ${requests.filter(p => p.startsWith('/api/')).length}`)
        if (repeat === 0 && cache === 'warm') await page.screenshot({ path: resolve(out, `${scenario.name}-${scenario.width}.png`), fullPage: false })
      }
      await context.close()
    }
  }
} finally {
  const version = browser.version()
  await browser.close()
  for (const stream of streams) stream.end()
  server.closeAllConnections()
  await new Promise(resolve => server.close(resolve))
  writeFileSync(resolve(out, 'browser.json'), JSON.stringify({
    sourceCommit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceStatus: execFileSync('git', ['status', '--short', '--', 'frontend'], { cwd: root, encoding: 'utf8' }).trim(),
    assetHashes: Object.fromEntries(readdirSync(resolve(root, 'frontend/dist/assets')).map(name => [name, createHash('sha256').update(readFileSync(resolve(root, 'frontend/dist/assets', name))).digest('hex')])),
    browser: version, node: process.version, cpu: cpus()[0].model, logicalCpus: cpus().length, memoryGiB: totalmem() / 2 ** 30,
    conditions: { latencyMs: 100, downloadBytesPerSecond: 200000, uploadBytesPerSecond: 93750, cpuSlowdown: 4, gzip: true, streamDelayMs, api: 'HTTP synthetic fixtures; no backend/database latency', cold: 'fresh browser context, empty HTTP cache', warm: 'same-context full navigation, cached immutable assets, new JS/query cache', observeAfterReadyMs: 700 },
    errors, unexpected, samples,
  }, null, 2) + '\n')
}
if (errors.length || unexpected.length) throw new Error(JSON.stringify({ errors, unexpected }))
