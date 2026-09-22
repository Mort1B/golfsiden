import { readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { resolve } from 'node:path'
const evidence = JSON.parse(readFileSync(process.argv[2], 'utf8')), out = process.argv[3]
mkdirSync(out, { recursive: true })
const median = values => { const sorted = [...values].sort((a,b) => a-b); return sorted[Math.floor(sorted.length / 2)] }
const range = values => ({ median: median(values), min: Math.min(...values), max: Math.max(...values) })
if (evidence.samples.length !== 54 || evidence.probes.some(p => !p.parity) || evidence.samples.some(s => s.errors.length || s.serverErrors.length || s.metrics.overflow)) throw new Error('Incomplete/failed final evidence')
const payloads = evidence.payloads.map(p => ({ ...p, inventoryEpoch: 0, removedRawPercent: 100 * (1 - p.playerSubset.rawBytes / p.full.rawBytes), removedGzipPercent: 100 * (1 - p.playerSubset.gzipBytes / p.full.gzipBytes), detailArrayPercent: 100 * p.detailArraysRawBytes / p.full.rawBytes,
  probe: Object.fromEntries(['full', 'subset'].map(name => [name, Object.fromEntries(['parseMs','decodeMs','filterMs'].map(metric => [metric, range(evidence.probes.find(v => v.scenario === p.scenario).measurements.filter(v => v.name === name).map(v => v[metric]))]))])),
}))
const keys = [...new Set(evidence.samples.map(s => `${s.scenario}/${s.width}/${s.route}`))]
const routes = keys.map(key => {
  const samples = evidence.samples.filter(s => `${s.scenario}/${s.width}/${s.route}` === key)
  if (samples.length !== 3) throw new Error(`Missing repetitions: ${key}`)
  return { key, repeats: samples.length, cards: samples[0].metrics.cards, rows: samples[0].metrics.rows,
    metrics: Object.fromEntries(['eventToDomMs', 'responseToDomMs', 'jsonToDomMs', 'matchDomNodes'].map(metric => [metric, range(samples.map(s => s.metrics[metric]))])),
    listEncodedBytes: range(samples.map(s => s.metrics.resources.filter(r => r.path.endsWith('/matches')).reduce((n,r) => n+r.encodedBodySize,0))),
  }
})
const { probes, samples, payloads: originalPayloads, ...meta } = evidence
void probes; void originalPayloads
writeFileSync(resolve(out, 'summary.json'), JSON.stringify({ ...meta, sampleCount: samples.length, payloads, routes }, null, 2) + '\n')
writeFileSync(resolve(out, 'evidence.json'), JSON.stringify(evidence, null, 2) + '\n')
const header = 'scenario,route,width,repeat,cards,rows,eventToDomMs,lastListResponseToDomMs,lastListJsonToDomMs,matchDomNodes,listRawBytes,listGzipBytes'
writeFileSync(resolve(out, 'samples.csv'), header + '\n' + samples.map(s => [s.scenario,s.route,s.width,s.repeat,s.metrics.cards,s.metrics.rows,s.metrics.eventToDomMs,s.metrics.responseToDomMs,s.metrics.jsonToDomMs,s.metrics.matchDomNodes,...['decodedBodySize','encodedBodySize'].map(field => s.metrics.resources.filter(r => r.path.endsWith('/matches')).reduce((n,r) => n+r[field],0))].join(',')).join('\n') + '\n')
console.log(`${samples.length} browser cases, ${payloads.length} paired payload/decoder scenarios retained`)
