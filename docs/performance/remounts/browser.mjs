import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { readFileSync, writeFileSync, mkdirSync, readdirSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { startFixture } from './server.mjs'
import { installObserver } from './observe.mjs'
import { tournamentId, playerId } from '../fixtures.mjs'
const root = fileURLToPath(new URL('../../../', import.meta.url))
const { chromium } = createRequire(resolve(root, 'frontend/package.json'))('playwright')
const out = process.argv[2] ?? '/tmp/match-remounts'
mkdirSync(out, { recursive: true })
const fixture = await startFixture(root), browser = await chromium.launch({ channel: 'chrome', headless: true }), samples = []
const routes = {
  direct: `/tournaments/${tournamentId}/match-results`,
  history: `/tournaments/${tournamentId}/results/players/${playerId}?metric=net`,
  global: `/leaderboard?tournament=${tournamentId}&scope=tournament&metric=net`,
}
async function until(test, label) {
  const deadline = Date.now() + 20000
  while (!test()) { if (Date.now() > deadline) throw new Error(`Timeout: ${label}`); await new Promise(done => setTimeout(done, 20)) }
}
try {
  for (const width of (process.env.REMOUNT_WIDTH ? [Number(process.env.REMOUNT_WIDTH)] : [320, 390, 1280])) {
    for (const [route, path] of Object.entries(routes).filter(([name]) => !process.env.REMOUNT_ROUTE || name === process.env.REMOUNT_ROUTE)) {
      for (const initial of (process.env.REMOUNT_INITIAL ? [process.env.REMOUNT_INITIAL] : ['settled', 'pending'])) {
        const repeat = initial === 'settled' ? 1 : 2
        const context = await browser.newContext({ viewport: { width, height: width === 320 ? 600 : width === 390 ? 844 : 900 } })
        const page = await context.newPage(), cdp = await context.newCDPSession(page), state = fixture.begin()
        const requests = [], pending = new Map(), errors = [], phases = []
        const sample = { route, width, repeat, initial, requests, errors, phases, server: state.requests, expectedDenials: [] }
        samples.push(sample)
        page.setDefaultTimeout(20000)
        await cdp.send('Network.enable')
        await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 100, downloadThroughput: 200000, uploadThroughput: 93750 })
        await cdp.send('Emulation.setCPUThrottlingRate', { rate: 4 })
        page.on('request', req => {
          const row = { id: requests.length + 1, path: new URL(req.url()).pathname, phase: state.phase, startMs: Date.now() - state.origin }
          requests.push(row); pending.set(req, row)
        })
        page.on('requestfinished', req => { const row = pending.get(req); if (row) row.finishMs = Date.now() - state.origin; pending.delete(req) })
        page.on('requestfailed', req => {
          const row = pending.get(req), reason = req.failure()?.errorText
          if (row) { row.failMs = Date.now() - state.origin; row.failure = reason }
          if (!new URL(req.url()).pathname.endsWith('/live') && reason !== 'net::ERR_ABORTED') errors.push(reason)
          pending.delete(req)
        })
        page.on('pageerror', error => errors.push(error.message))
        page.on('console', message => { if (message.type() === 'error' && !/^Failed to load resource: the server responded with a status of (401|403|404)/.test(message.text())) errors.push(message.text()) })
        page.on('response', res => { if (res.status() < 400) return
          const path = new URL(res.url()).pathname
          if (path === state.denyPath && res.status() === state.denyStatus) sample.expectedDenials.push({ path, status: res.status(), phase: state.phase })
          else errors.push(`HTTP ${res.status()} ${path}`)
        })
        await page.addInitScript(installObserver, state.origin)
        const expected = route.includes('history') ? { cards: 3, rows: 1 } : { cards: 72, rows: 48 }
        const mark = async name => {
          state.phase = name
          const eventIndex = await page.evaluate(() => window.subscriberTrace.length)
          phases.push({ name, atMs: Date.now() - state.origin, eventIndex })
        }
        const ready = async () => {
          await page.waitForFunction(({ epoch, expected }) => {
            const cards = [...document.querySelectorAll('.match-list article h3')], rows = [...document.querySelectorAll('.match-table li strong')]
            return cards.length === expected.cards && rows.length === expected.rows && [...cards, ...rows].every(el => el.textContent.includes(`[epoch ${epoch}]`))
          }, { epoch: state.epoch, expected })
          await page.waitForTimeout(700)
          await until(() => [...pending.values()].every(r => r.path.endsWith('/live')), 'HTTP settling')
        }
        if (initial === 'pending') state.holdKinds = ['matches']
        await page.goto(`${fixture.base}${path}`, { waitUntil: 'domcontentloaded' })
        if (initial === 'pending') await until(() => state.requests.filter(r => r.held).length >= 3, 'initial held lists')
        else await ready()
        await mark('open'); state.epoch = 1; state.holdKinds = []; fixture.open()
        await page.waitForTimeout(400); fixture.release(); await ready()
        await mark('visibility'); state.epoch = 2; state.holdKinds = ['match-table']; fixture.visibility()
        await until(() => state.requests.some(r => r.held && r.epoch === 2), 'held visibility table')
        await page.waitForFunction(() => document.querySelectorAll('.match-list article, .match-table li').length === 0)
        await page.waitForTimeout(400)
        await mark('table-release'); state.holdKinds = []; fixture.release(); await ready()
        await mark('pending-match'); state.epoch = 3; state.holdKinds = ['matches']; fixture.match()
        await until(() => state.requests.filter(r => r.held && r.epoch === 3).length >= 3, 'old pending lists')
        await mark('visibility-pending'); state.epoch = 4; state.holdKinds = ['match-table']; fixture.visibility()
        await until(() => state.requests.some(r => r.held && r.epoch === 4), 'next held visibility table')
        await page.waitForFunction(() => document.querySelectorAll('.match-list article, .match-table li').length === 0)
        await page.waitForTimeout(400)
        await mark('pending-release'); state.holdKinds = []; fixture.release(); await ready()
        await mark('held-match'); state.epoch = 5; state.holdKinds = ['matches', 'match-table']; fixture.match()
        await until(() => state.requests.filter(r => r.held && r.epoch === 5).length >= 4, 'held table and three lists')
        await mark('error'); fixture.cut()
        await page.waitForFunction(() => window.subscriberTrace.some(e => e.type === 'dispatch-start' && e.signal === 'error') && document.querySelectorAll('.match-list article, .match-table li').length === 0)
        await until(() => state.requests.filter(r => r.path.endsWith('/live')).length >= 2, 'native reconnect')
        await mark('reconnect'); state.epoch = 6; state.holdKinds = []; fixture.open()
        await page.waitForTimeout(400); fixture.release(); await ready()
        await mark('resume'); state.epoch = 7
        await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })))
        await ready()
        await mark('account-held'); state.epoch = 8; state.holdKinds = ['matches', 'match-table']; fixture.match()
        await until(() => state.requests.filter(r => r.held && r.epoch === 8).length >= 4, 'held old account')
        await mark('account'); state.account = 2; state.epoch = 9; state.holdKinds = []
        await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })))
        await page.waitForTimeout(400); fixture.release(); await ready()
        // Each width selects a different dependency/status; no claim of a full cross-product.
        const dependency = width === 320 ? 'match-table' : width === 390 ? 'rounds' : 'matches'
        state.denyPath = dependency === 'matches' ? `/api/rounds/${state.requests.find(r => r.path.endsWith('/matches')).path.split('/')[3]}/match-play/matches` : `/api/tournaments/${tournamentId}/${dependency}`
        state.denyStatus = width === 320 ? 403 : width === 390 ? 401 : 404
        await mark('denial'); fixture.visibility()
        await page.getByText('Synthetic access denied').first().waitFor()
        await page.waitForFunction(() => document.querySelectorAll('.match-list article, .match-table li').length === 0)
        sample.deniedPublishedIndex = await page.evaluate(() => window.subscriberTrace.length)
        await page.waitForTimeout(700)
        await until(() => [...pending.values()].every(r => r.path.endsWith('/live')), 'denial settling')
        if (sample.expectedDenials.length === 0) errors.push('missing expected denial')
        const deniedEvents = await page.evaluate(() => window.subscriberTrace)
        const firstDenied = deniedEvents.findIndex(e => e.type === 'content' && e.denied)
        if (firstDenied < 0 || deniedEvents.slice(firstDenied).some(e => e.type === 'content' && (e.cards || e.rows))) errors.push('content alongside or after rendered denial')
        if (deniedEvents.slice(sample.deniedPublishedIndex).some(e => e.type === 'content' && (e.cards || e.rows))) errors.push('content restored after visible denial')
        await page.screenshot({ path: resolve(out, `denial-${route}-${width}-${initial}.png`) })
        await mark('recovery'); state.denyPath = ''; state.epoch = 10; fixture.visibility(); await ready()
        const events = await page.evaluate(() => window.subscriberTrace)
        const afterSignal = (signal, epoch, from = 0) => {
          const index = events.findIndex((e, i) => i >= from && e.type === 'dispatch-start' && e.signal === signal)
          if (index < 0 || events.slice(index).some(e => e.type === 'content' && e.epochs.some(n => n < epoch))) errors.push(`stale after ${signal}/${epoch}`)
        }
        afterSignal('open', 1)
        afterSignal('visibility', 2)
        afterSignal('visibility', 4, phases.find(p => p.name === 'visibility-pending').eventIndex)
        afterSignal('error', 6)
        const accountIndex = phases.find(p => p.name === 'account').eventIndex
        const freshAccount = events.findIndex((e, i) => i >= accountIndex && e.type === 'content' && e.epochs.includes(9))
        if (freshAccount < 0 || events.slice(freshAccount).some(e => e.type === 'content' && e.epochs.some(n => n < 9))) errors.push('stale account repaint')
        const clearedAccount = events.findIndex((e, i) => i >= accountIndex && e.type === 'content' && e.cards === 0 && e.rows === 0)
        if (clearedAccount < 0 || clearedAccount >= freshAccount || events.slice(clearedAccount).some(e => e.type === 'content' && e.epochs.some(n => n < 9))) errors.push('old account content after clearing')
        if (state.requests.filter(r => r.held && ![2, 4].includes(r.epoch)).some(r => !r.aborted || !(r.closeMs < r.releaseMs) || r.finishMs !== undefined)) errors.push('obsolete held read not cancelled before release')
        if (state.requests.filter(r => r.held && [2, 4].includes(r.epoch)).some(r => r.finishMs === undefined || r.aborted)) errors.push('required held table failed')
        const sources = events.filter(e => e.type === 'source-create'), closes = events.filter(e => e.type === 'source-close')
        if (sources.length !== 2 || closes.length !== 1 || sources[0].source !== closes[0].source) errors.push('unexpected source ownership')
        const replacement = events.findIndex(e => e === sources[1])
        if (replacement < 0 || events.slice(replacement).some(e => e.type === 'content' && e.epochs.some(n => n < 9))) errors.push('old content after replacement source')
        if (requests.filter(r => r.path.endsWith('/live')).length !== 3) errors.push('unexpected stream count')
        for (const phase of ['resume', 'account']) {
          if (requests.filter(r => r.phase === phase && r.path.endsWith('/session')).length !== 1) errors.push(`uncoalesced auth ${phase}`)
        }
        const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)
        Object.assign(sample, { events, overflow, serverErrors: state.errors, finalEpoch: state.epoch, expected, pendingNonSse: [...pending.values()].filter(r => !r.path.endsWith('/live')).length })
        if (overflow || errors.length || state.errors.length) throw new Error(JSON.stringify({ route, width, errors, serverErrors: state.errors, overflow }))
        await page.screenshot({ path: resolve(out, `${route}-${width}-${initial}.png`) })
        fixture.endSample(); await context.close()
        console.log(`${route} ${width} ${initial} passed`)
      }
    }
  }
} finally {
  const version = browser.version()
  await browser.close(); await fixture.close()
  writeFileSync(resolve(out, 'trace.json'), JSON.stringify({ measuredAt: new Date().toISOString(), sourceCommit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceStatus: execFileSync('git', ['status', '--short', '--', 'frontend'], { cwd: root, encoding: 'utf8' }).trim(), browser: version, node: process.version,
    assetHashes: Object.fromEntries(readdirSync(resolve(root, 'frontend/dist/assets')).map(name => [name, createHash('sha256').update(readFileSync(resolve(root, 'frontend/dist/assets', name))).digest('hex')])),
    conditions: { rounds: 3, matchesPerRound: 24, latencyMs: 100, downloadBytesPerSecond: 200000, uploadBytesPerSecond: 93750, cpuSlowdown: 4, settleMs: 700, retryMs: 200, api: 'synthetic native HTTP and SSE; no backend/database', cache: 'fresh context per navigation', instrumentation: 'native fetch/EventSource callback wrappers and DOM metadata; no query-cache or production source patch' }, samples }, null, 2) + '\n')
}
