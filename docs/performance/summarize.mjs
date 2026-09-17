import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
const input = JSON.parse(readFileSync(process.argv[2], 'utf8'))
const destination = process.argv[3] ?? '/tmp/golf-performance'
const groups = new Map()
const rows = input.samples.map(s => {
  const resources = s.resources
  const sum = (test, field) => resources.filter(r => test(r.path)).reduce((n, r) => n + r[field], 0)
  const row = {
    scenario: s.name, width: s.width, repeat: s.repeat, cache: s.cache,
    readyMs: Math.round(s.readyMs), fcpMs: Math.round(s.fcpMs), lcpObservedMs: Math.round(s.lcpMs),
    longTasks: s.longTaskCount, longTaskMs: s.longTaskMs, tbtProxyMs: s.tbtProxyMs,
    apiRequests: s.requests.filter(p => p.startsWith('/api/') && !p.endsWith('/live')).length,
    liveRequests: s.requests.filter(p => p.endsWith('/live')).length,
    listRequests: s.requests.filter(p => p.endsWith('/matches')).length,
    listEncodedBytes: sum(p => p.endsWith('/matches'), 'encodedBytes'),
    listDecodedBytes: sum(p => p.endsWith('/matches'), 'decodedBytes'),
    jsTransferBytes: sum(p => p.endsWith('.js'), 'transferBytes'),
    cssTransferBytes: sum(p => p.endsWith('.css'), 'transferBytes'),
    domNodes: s.domNodes, overflow: s.overflow, finalCards: s.finalCards, finalRows: s.finalRows,
    pendingRequests: s.pendingPaths.length,
  }
  const key = `${s.name}-${s.width}-${s.cache}`
  if (!groups.has(key)) groups.set(key, [])
  groups.get(key).push(row)
  return row
})
function distribution(values) {
  values.sort((a, b) => a - b)
  const mid = Math.floor(values.length / 2)
  return { min: values[0], median: values.length % 2 ? values[mid] : (values[mid - 1] + values[mid]) / 2, max: values.at(-1) }
}
const summary = Object.fromEntries([...groups].map(([key, values]) => [key, {
  samples: values.length,
  ...Object.fromEntries(['readyMs', 'fcpMs', 'lcpObservedMs', 'tbtProxyMs', 'apiRequests', 'liveRequests', 'listRequests', 'listEncodedBytes', 'listDecodedBytes', 'domNodes'].map(field => [field, distribution(values.map(v => v[field]))])),
}]))
const { samples: _samples, ...metadata } = input
writeFileSync(resolve(destination, 'summary.json'), JSON.stringify({ ...metadata, summary }, null, 2) + '\n')
const headers = Object.keys(rows[0])
writeFileSync(resolve(destination, 'samples.csv'), headers.join(',') + '\n' + rows.map(row => headers.map(h => row[h]).join(',')).join('\n') + '\n')
console.log(JSON.stringify(summary, null, 2))
