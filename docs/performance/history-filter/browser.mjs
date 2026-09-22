import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { readFileSync, writeFileSync, mkdirSync, readdirSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { gzipSync } from 'node:zlib'
import { startFixture } from './server.mjs'
import { scenarios, dataFor, playerSubset, playerId, tournamentId } from './fixtures.mjs'
const root = fileURLToPath(new URL('../../../', import.meta.url))
const require = createRequire(resolve(root, 'frontend/package.json'))
const { chromium } = require('playwright'), { build } = require('esbuild')
const out = process.argv[2] ?? '/tmp/history-payload'
mkdirSync(out, { recursive: true })
const bundled = await build({ entryPoints: [resolve(root, 'docs/performance/history-filter/probe.mjs')], bundle: true, minify: true, write: false, format: 'iife', platform: 'browser' })
const probeCode = bundled.outputFiles[0].contents
const fixture = await startFixture(root, probeCode), browser = await chromium.launch({ channel: 'chrome', headless: true })
const samples = [], probes = [], payloads = []
const routes = {
  all: `/tournaments/${tournamentId}/match-results`,
  filtered: `/tournaments/${tournamentId}/match-results?player=${playerId}`,
  history: `/tournaments/${tournamentId}/results/players/${playerId}?metric=net`,
}
const selected = scenarios.filter(s => !process.env.HISTORY_CASE || s.name === process.env.HISTORY_CASE)
const hash = value => createHash('sha256').update(value).digest('hex')
async function throttle(context, page) {
  const cdp = await context.newCDPSession(page)
  await cdp.send('Network.enable')
  await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 100, downloadThroughput: 200000, uploadThroughput: 93750 })
  await cdp.send('Emulation.setCPUThrottlingRate', { rate: 4 })
}
async function expectStream(state) {
  const deadline = Date.now() + 20000
  while (!state.openers.length) {
    if (Date.now() > deadline) throw new Error('Missing native stream connection')
    await new Promise(done => setTimeout(done, 20))
  }
}
try {
  for (const scenario of selected) {
    const data = dataFor(scenario), full = [...data.listings.values()], subset = full.map(l => ({ ...playerSubset(l), player_id: playerId }))
    const size = listings => ({ rawBytes: listings.reduce((n, l) => n + Buffer.byteLength(JSON.stringify(l)), 0), gzipBytes: listings.reduce((n, l) => n + gzipSync(JSON.stringify(l)).length, 0), cards: listings.reduce((n, l) => n + l.matches.length, 0) })
    payloads.push({ scenario: scenario.name, full: size(full), playerSubset: size(subset), table: { rawBytes: Buffer.byteLength(JSON.stringify(data.table)), gzipBytes: gzipSync(JSON.stringify(data.table)).length },
      fixtureHash: hash(JSON.stringify({ rounds: data.rounds, full, table: data.table })),
      detailArraysRawBytes: full.reduce((sum, l) => sum + l.matches.reduce((n, c) => n + ['holes', 'notes', 'events'].reduce((total, field) => total + Buffer.byteLength(JSON.stringify(c[field])), 0), 0), 0) })
    // Isolated, warmed browser decoder batches: same strict decoder, no network or React.
    fixture.begin(scenario)
    const probeContext = await browser.newContext(), probePage = await probeContext.newPage()
    await throttle(probeContext, probePage)
    await probePage.goto(`${fixture.base}/probe`); await probePage.addScriptTag({ url: `${fixture.base}/probe.js` })
    const measurements = await probePage.evaluate(({ full, subset, playerId }) => {
      const variants = { full, subset }, rows = [], iterations = 20
      const run = (name, lists, batch) => {
        const strings = lists.map(l => JSON.stringify(l))
        let t = performance.now(), parsed
        for (let i = 0; i < iterations; i++) parsed = strings.map(s => JSON.parse(s))
        const parseMs = (performance.now() - t) / iterations
        t = performance.now(); let decoded
        for (let i = 0; i < iterations; i++) decoded = parsed.map(l => (name === 'subset' ? window.decodePlayerListingProbe(l, l.round_id, playerId) : window.decodeListingProbe(l, l.round_id)))
        const decodeMs = (performance.now() - t) / iterations
        t = performance.now(); let selected
        for (let i = 0; i < iterations; i++) selected = decoded.map(l => l.matches.filter(m => m.opponents.some(p => p.player_id === playerId)))
        const filterMs = (performance.now() - t) / iterations
        if (selected.some(cards => cards.length !== 1)) throw new Error('Unexpected selected-card count')
        return { name, batch, iterations, parseMs, decodeMs, filterMs }
      }
      for (const [name, lists] of Object.entries(variants)) run(name, lists, -1)
      for (let batch = 0; batch < 7; batch++) for (const name of batch % 2 ? ['subset', 'full'] : ['full', 'subset']) rows.push(run(name, variants[name], batch))
      const decoded = full.map(l => window.decodeListingProbe(l, l.round_id))
      const narrowed = subset.map(l => { const { player_id, ...listing } = window.decodePlayerListingProbe(l, l.round_id, playerId); if (player_id !== playerId) throw new Error('Wrong player'); return listing })
      if (JSON.stringify(decoded.map(l => ({ ...l, matches: l.matches.filter(m => m.opponents.some(p => p.player_id === playerId)), writable_match_ids: l.writable_match_ids.filter(id => l.matches.some(m => m.match_id === id && m.opponents.some(p => p.player_id === playerId))) }))) !== JSON.stringify(narrowed)) throw new Error('Subset semantic parity failed')
      return rows
    }, { full, subset, playerId })
    probes.push({ scenario: scenario.name, measurements, parity: true }); await probeContext.close()
    for (const width of scenario.widths) for (let repeat = 0; repeat < Number(process.env.HISTORY_REPEATS ?? 3); repeat++) {
      // Rotate route order across repetitions to reduce systematic ordering bias.
      const names = Object.keys(routes); const order = [...names.slice(repeat % 3), ...names.slice(0, repeat % 3)]
      for (const route of order) {
        const state = fixture.begin(scenario), errors = [], context = await browser.newContext({ viewport: { width, height: width === 320 ? 600 : 900 } })
        const page = await context.newPage(); await throttle(context, page)
        page.on('pageerror', e => errors.push(e.message))
        page.on('console', m => { if (m.type() === 'error') errors.push(m.text()) })
        page.on('response', r => { if (r.status() >= 400) errors.push(`HTTP ${r.status()} ${new URL(r.url()).pathname}`) })
        page.on('requestfailed', r => { if (r.failure()?.errorText !== 'net::ERR_ABORTED') errors.push(r.failure()?.errorText) })
        const expectedCards = scenario.rounds * (route === 'all' ? scenario.matches : 1), expectedRows = route === 'all' ? scenario.matches * 2 : 1
        await page.addInitScript(({ expectedCards, expectedRows }) => {
          window.historyProbe = { dom: {}, json: [] }
          const nativeFetch = window.fetch
          window.fetch = async (...args) => {
            const response = await nativeFetch(...args), path = new URL(response.url).pathname
            if (path.endsWith('/matches')) {
              const nativeJson = response.json.bind(response)
              response.json = async () => { const value = await nativeJson(); window.historyProbe.json.push({ path, at: performance.now() }); return value }
            }
            return response
          }
          const observe = () => {
            const nodes = [...document.querySelectorAll('.match-list article h3, .match-table li strong')]
            if (document.querySelectorAll('.match-list article').length !== expectedCards || document.querySelectorAll('.match-table li').length !== expectedRows) return
            for (const epoch of [0, 1, 2]) if (window.historyProbe.dom[epoch] === undefined && nodes.every(n => n.textContent.includes(`[epoch ${epoch}]`))) window.historyProbe.dom[epoch] = performance.now()
          }
          new MutationObserver(observe).observe(document, { subtree: true, childList: true, characterData: true })
        }, { expectedCards, expectedRows })
        await page.goto(fixture.base + routes[route], { waitUntil: 'domcontentloaded' })
        const ready = async epoch => {
          await page.waitForFunction(epoch => window.historyProbe.dom[epoch] !== undefined, epoch)
          await page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))))
        }
        await ready(0)
        await expectStream(state)
        fixture.open(); await ready(1)
        const mark = await page.evaluate(() => performance.now())
        fixture.refresh(); await ready(2)
        const metrics = await page.evaluate(mark => {
          const resources = performance.getEntriesByType('resource').filter(r => r.startTime >= mark && /\/api\/.*(matches|match-table)$/.test(new URL(r.name).pathname)).map(r => ({ path: new URL(r.name).pathname, query: new URL(r.name).search, startTime: r.startTime, responseStart: r.responseStart, responseEnd: r.responseEnd, transferSize: r.transferSize, encodedBodySize: r.encodedBodySize, decodedBodySize: r.decodedBodySize }))
          const lists = resources.filter(r => r.path.endsWith('/matches')), lastResponse = Math.max(...lists.map(r => r.responseEnd)), jsonReady = Math.max(...window.historyProbe.json.filter(r => r.at >= mark).map(r => r.at))
          return { mark, resources, domAt: window.historyProbe.dom[2], responseToDomMs: window.historyProbe.dom[2] - lastResponse, jsonToDomMs: window.historyProbe.dom[2] - jsonReady,
            eventToDomMs: window.historyProbe.dom[2] - mark, matchDomNodes: document.querySelector('.match-page')?.querySelectorAll('*').length,
            cards: document.querySelectorAll('.match-list article').length, rows: document.querySelectorAll('.match-table li').length, overflow: document.documentElement.scrollWidth > innerWidth }
        }, mark)
        const refresh = state.requests.filter(r => r.phase === 'refresh' && r.path.endsWith('/matches'))
        if (refresh.some(r => r.query !== (route === 'all' ? '' : `?player_id=${playerId}`) || r.cards !== (route === 'all' ? scenario.matches : 1))) throw new Error('Wrong wire filter/card count')
        const tables = state.requests.filter(r => r.phase === 'refresh' && r.path.endsWith('/match-table'))
        if (tables.length !== 1 || !tables[0].finished || tables[0].aborted || metrics.resources.filter(r => r.path.endsWith('/match-table')).length !== 1) throw new Error('Required table refresh mismatch')
        for (const row of [...refresh, ...tables]) {
          const resource = metrics.resources.find(r => r.path === row.path && r.query === row.query)
          if (!resource || resource.encodedBodySize !== row.gzipBytes || resource.decodedBodySize !== row.rawBytes) throw new Error(`Resource body byte mismatch ${row.path}`)
        }
        if (![metrics.responseToDomMs, metrics.jsonToDomMs, metrics.eventToDomMs].every(Number.isFinite)) throw new Error('Missing timing evidence')
        if (refresh.length !== scenario.rounds || refresh.some(r => !r.finished || r.aborted) || metrics.resources.filter(r => r.path.endsWith('/matches')).length !== scenario.rounds || metrics.cards !== expectedCards || metrics.rows !== expectedRows || metrics.overflow || errors.length || state.errors.length) throw new Error(JSON.stringify({ scenario, route, width, metrics, refresh, errors, serverErrors: state.errors }))
        if (repeat === 0) {
          await page.screenshot({ path: resolve(out, `${scenario.name}-${route}-${width}.png`) })
          await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight))
          await page.screenshot({ path: resolve(out, `${scenario.name}-${route}-${width}-bottom.png`) })
        }
        samples.push({ scenario: scenario.name, route, width, repeat, expectedCards, metrics, requests: state.requests.filter(r => r.path.startsWith('/api/')), errors, serverErrors: state.errors })
        await context.close(); console.log(`${scenario.name} ${route} ${width} repeat ${repeat + 1} passed`)
      }
    }
  }
} finally {
  const version = browser.version(); await browser.close(); await fixture.close()
  writeFileSync(resolve(out, 'evidence.json'), JSON.stringify({ measuredAt: new Date().toISOString(), sourceCommit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), sourceStatus: execFileSync('git', ['status', '--short', '--', 'frontend', 'backend'], { cwd: root, encoding: 'utf8' }).trim(), browser: version, node: process.version,
    conditions: { latencyMs: 100, downloadBytesPerSecond: 200000, uploadBytesPerSecond: 93750, cpuSlowdown: 4, repeats: Number(process.env.HISTORY_REPEATS ?? 3), api: 'native synthetic gzip HTTP and SSE; no backend/database', timing: 'match-event refresh after settled initial and open authority passes; isolated warmed decoder batches separately; candidate production build with player-filtered full-card endpoint' }, probeHash: hash(probeCode),
    assetHashes: Object.fromEntries(readdirSync(resolve(root, 'frontend/dist/assets')).map(name => [name, hash(readFileSync(resolve(root, 'frontend/dist/assets', name)))])), payloads, probes, samples }, null, 2) + '\n')
}
