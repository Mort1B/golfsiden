import { readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { resolve } from 'node:path'
const input = JSON.parse(readFileSync(process.argv[2], 'utf8'))
const out = process.argv[3]
mkdirSync(out, { recursive: true })
const isList = path => path.endsWith('/matches'), isTable = path => path.endsWith('/match-table')
const rows = input.samples.map(s => {
  if (s.errors.length || s.serverErrors.length || s.pendingNonSse || s.overflow || s.cards !== 72 || s.rows !== 48) throw new Error('Incomplete or failed sample')
  const requests = s.requests.filter(r => isList(r.path)), table = s.requests.filter(r => isTable(r.path))
  const calls = s.events.filter(e => e.type === 'fetch-start' && (isList(e.path) || isTable(e.path)))
  const held = s.server.filter(r => r.held)
  return { mode: s.mode, width: s.width, repeat: s.repeat, cache: s.cache,
    listStarts: requests.length, listFinished: requests.filter(r => r.finishMs !== undefined).length,
    listAborted: requests.filter(r => r.failure === 'net::ERR_ABORTED').length,
    tableStarts: table.length, tableFinished: table.filter(r => r.finishMs !== undefined).length,
    tableAborted: table.filter(r => r.failure === 'net::ERR_ABORTED').length,
    heldLists: held.filter(r => isList(r.path)).length, heldTables: held.filter(r => isTable(r.path)).length,
    listTableCallsWithSignal: calls.filter(e => e.hasSignal).length,
    listGzipBytes: s.resources.filter(r => isList(r.path)).reduce((sum, r) => sum + r.encodedBytes, 0),
    tableGzipBytes: s.resources.filter(r => isTable(r.path)).reduce((sum, r) => sum + r.encodedBytes, 0),
    sseOpen: s.events.filter(e => e.type === 'sse-open').length, sseError: s.events.filter(e => e.type === 'sse-error').length,
    cards: s.cards, rows: s.rows, finalEpoch: s.finalEpoch, pendingNonSse: s.pendingNonSse, errors: s.errors.length + s.serverErrors.length }
})
const groups = {}
for (const row of rows) {
  const key = `${row.mode}-${row.width}-${row.cache}`
  const group = groups[key] ??= { samples: 0 }
  group.samples++
  for (const field of ['listStarts', 'listFinished', 'listAborted', 'tableStarts', 'tableFinished', 'tableAborted', 'listGzipBytes', 'tableGzipBytes', 'listTableCallsWithSignal']) {
    const range = group[field] ??= { min: row[field], max: row[field] }
    range.min = Math.min(range.min, row[field]); range.max = Math.max(range.max, row[field])
  }
}
const { samples, ...metadata } = input
writeFileSync(resolve(out, 'summary.json'), JSON.stringify({ ...metadata, sampleCount: samples.length, groups }, null, 2) + '\n')
const headers = Object.keys(rows[0])
writeFileSync(resolve(out, 'samples.csv'), headers.join(',') + '\n' + rows.map(r => headers.map(h => r[h]).join(',')).join('\n') + '\n')
// Keep all API request/DOM observations, omitting asset request rows (hashes above).
writeFileSync(resolve(out, 'traces.jsonl'), samples.map(s => JSON.stringify({ ...s,
  server: s.server.filter(r => r.path.startsWith('/api/')), requests: s.requests.filter(r => r.path.startsWith('/api/')) })).join('\n') + '\n')
console.log(`Retained ${samples.length} validated samples and their API traces.`)
