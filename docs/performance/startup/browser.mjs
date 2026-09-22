import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { readFileSync, writeFileSync, mkdirSync, readdirSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { cpus } from 'node:os'
import { startFixture } from './server.mjs'
import { installObserver } from './observe.mjs'
import { tournamentId } from '../fixtures.mjs'
const root = fileURLToPath(new URL('../../../', import.meta.url))
const { chromium } = createRequire(resolve(root, 'frontend/package.json'))('playwright')
const out = process.argv[2] ?? '/tmp/match-startup'
mkdirSync(out, { recursive: true })
const fixture = await startFixture(root), browser = await chromium.launch({ channel: 'chrome', headless: true })
const samples = []
async function until(test, label) {
  const deadline = Date.now() + 15000
  while (!test()) {
    if (Date.now() > deadline) throw new Error(`Timeout: ${label}`)
    await new Promise(done => setTimeout(done, 20))
  }
}
async function ready(page, epoch) {
  await page.waitForFunction(epoch => {
    const cards = [...document.querySelectorAll('.match-list article h3')], rows = [...document.querySelectorAll('.match-table li strong')]
    return cards.length === 72 && rows.length === 48 && [...cards, ...rows].every(el => el.textContent.includes(`[epoch ${epoch}]`))
  }, epoch)
}
try {
  for (const width of (process.env.STARTUP_WIDTH ? [Number(process.env.STARTUP_WIDTH)] : [320, 390, 1280])) {
    for (const mode of (process.env.STARTUP_MODE ? [process.env.STARTUP_MODE] : ['natural', 'late-open', 'overlap', 'reconnect'])) {
      for (let repeat = 1; repeat <= Number(process.env.STARTUP_REPEATS ?? 2); repeat++) {
        const context = await browser.newContext({ viewport: { width, height: width === 320 ? 600 : width === 390 ? 844 : 900 } })
        for (const cache of ['cold', 'warm']) {
          const page = await context.newPage(), cdp = await context.newCDPSession(page)
          page.setDefaultTimeout(15000)
          await cdp.send('Network.enable')
          await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 100, downloadThroughput: 200000, uploadThroughput: 93750 })
          await cdp.send('Emulation.setCPUThrottlingRate', { rate: 4 })
          const state = fixture.begin(mode), requests = [], pending = new Map(), errors = []
          const sample = { width, mode, repeat, cache, errors, server: state.requests, requests }
          samples.push(sample)
          const start = req => {
            const row = { id: requests.length + 1, path: new URL(req.url()).pathname, startMs: Date.now() - state.origin }
            requests.push(row); pending.set(req, row)
          }
          const finish = req => { const row = pending.get(req); if (row) row.finishMs = Date.now() - state.origin; pending.delete(req) }
          const fail = req => {
            const row = pending.get(req), reason = req.failure()?.errorText
            if (row) { row.failMs = Date.now() - state.origin; row.failure = reason }
            if (!new URL(req.url()).pathname.endsWith('/live') && reason !== 'net::ERR_ABORTED') errors.push(reason)
            pending.delete(req)
          }
          const consoleError = message => { if (message.type() === 'error') errors.push(message.text()) }
          const pageError = error => errors.push(error.message)
          const response = res => { if (res.status() >= 400) errors.push(`HTTP ${res.status()} ${new URL(res.url()).pathname}`) }
          page.on('request', start); page.on('requestfinished', finish); page.on('requestfailed', fail)
          page.on('console', consoleError); page.on('pageerror', pageError); page.on('response', response)
          // A new page per cache phase prevents accumulating observers; HTTP cache remains in the context.
          await page.addInitScript(installObserver, state.origin)
          await page.goto(`${fixture.base}/tournaments/${tournamentId}/match-results`, { waitUntil: 'domcontentloaded' })
          if (mode === 'late-open') {
            await ready(page, 0); await page.waitForTimeout(400); fixture.open()
          } else if (mode === 'overlap') {
            await until(() => state.requests.filter(r => r.held && r.path.endsWith('/matches')).length >= 3, 'three held lists')
            fixture.open()
            await page.waitForTimeout(400); fixture.release(); await ready(page, 1)
          }
          await ready(page, state.epoch)
          await page.waitForTimeout(700)
          if (mode === 'reconnect') {
            fixture.match()
            await until(() => state.requests.filter(r => r.held && r.path.endsWith('/matches')).length >= 3, 'match-triggered held lists')
            fixture.cut()
            await page.waitForFunction(() => window.startupTrace.some(e => e.type === 'sse-error') && document.querySelectorAll('.match-list article').length === 0 && document.querySelectorAll('.match-table li').length === 0)
            await until(() => state.requests.filter(r => r.path.endsWith('/live')).length >= 2, 'native reconnect')
            fixture.open()
            await page.waitForTimeout(400); fixture.release(); await ready(page, 2)
            await page.waitForTimeout(700)
          }
          await until(() => [...pending.values()].every(r => r.path.endsWith('/live')), 'settled HTTP')
          await ready(page, state.epoch)
          const result = await page.evaluate(() => ({ events: window.startupTrace, cards: document.querySelectorAll('.match-list article').length,
            rows: document.querySelectorAll('.match-table li').length, overflow: document.documentElement.scrollWidth > innerWidth,
            resources: performance.getEntriesByType('resource').filter(r => new URL(r.name).pathname.startsWith('/api/')).map(r => ({ path: new URL(r.name).pathname, startMs: r.startTime, durationMs: r.duration, transferBytes: r.transferSize, encodedBytes: r.encodedBodySize, decodedBytes: r.decodedBodySize })) }))
          const firstFresh = result.events.findIndex(e => e.type === 'content' && e.cards === 72 && e.rows === 48 && e.epochs.length === 1 && e.epochs[0] === state.epoch)
          if (firstFresh < 0 || result.events.slice(firstFresh).some(e => e.type === 'content' && e.epochs.some(epoch => epoch !== state.epoch))) errors.push('Stale repaint after fresh content')
          const openIndex = result.events.findIndex(e => e.type === 'sse-open')
          if (openIndex < 0 || result.events.slice(openIndex).some(e => e.type === 'content' && e.epochs.some(epoch => epoch < 1))) errors.push('Stale repaint after initial stream open')
          if (mode === 'reconnect') {
            const errorIndex = result.events.findIndex(e => e.type === 'sse-error')
            if (errorIndex < 0 || result.events.slice(errorIndex).some(e => e.type === 'content' && e.epochs.some(epoch => epoch < 2))) errors.push('Stale repaint after disconnect')
          }
          Object.assign(sample, result, { serverErrors: state.errors, finalEpoch: state.epoch, pendingNonSse: [...pending.values()].filter(r => !r.path.endsWith('/live')).length })
          if (result.overflow || errors.length || state.errors.length) throw new Error(JSON.stringify({ width, mode, errors, serverErrors: state.errors, overflow: result.overflow }))
          if (repeat === 1 && cache === 'warm') await page.screenshot({ path: resolve(out, `${mode}-${width}.png`) })
          page.off('request', start); page.off('requestfinished', finish); page.off('requestfailed', fail)
          page.off('console', consoleError); page.off('pageerror', pageError); page.off('response', response)
          fixture.endSample()
          console.log(`${mode} ${width} ${repeat} ${cache}: lists=${requests.filter(r => r.path.endsWith('/matches')).length}, held=${state.requests.filter(r => r.held).length}`)
          await page.close()
        }
        await context.close()
      }
    }
  }
} finally {
  const version = browser.version()
  await browser.close(); await fixture.close()
  writeFileSync(resolve(out, 'trace.json'), JSON.stringify({ measuredAt: new Date().toISOString(), sourceCommit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceStatus: execFileSync('git', ['status', '--short', '--', 'frontend'], { cwd: root, encoding: 'utf8' }).trim(), browser: version, node: process.version, cpu: cpus()[0].model,
    assetHashes: Object.fromEntries(readdirSync(resolve(root, 'frontend/dist/assets')).map(name => [name, createHash('sha256').update(readFileSync(resolve(root, 'frontend/dist/assets', name))).digest('hex')])),
    conditions: { matchesPerRound: 24, rounds: 3, latencyMs: 100, downloadBytesPerSecond: 200000, uploadBytesPerSecond: 93750, cpuSlowdown: 4, gzip: true,
      streamRetryMs: 200, holdAfterOpenMs: 400, settleMs: 700, cache: 'cold fresh context; warm new document with immutable cached assets', api: 'synthetic HTTP; held snapshots; no backend or database', instrumentation: 'native fetch/EventSource wrappers and DOM mutation metadata only' }, samples }, null, 2) + '\n')
}
