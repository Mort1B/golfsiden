import { readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { resolve } from 'node:path'
const trace = JSON.parse(readFileSync(process.argv[2], 'utf8')), out = process.argv[3]
mkdirSync(out, { recursive: true })
const counts = rows => ({ starts: rows.length, completed: rows.filter(r => r.finishMs !== undefined).length, aborted: rows.filter(r => r.failure === 'net::ERR_ABORTED').length })
const samples = trace.samples.map(sample => ({ route: sample.route, width: sample.width, repeat: sample.repeat, initial: sample.initial, expectedDenials: sample.expectedDenials,
  phases: ['initial', ...sample.phases.map(p => p.name)].map(name => {
    const requests = sample.requests.filter(r => r.phase === name)
    const start = sample.phases.find(p => p.name === name)?.eventIndex ?? 0
    const next = sample.phases.findIndex(p => p.name === name) + 1
    const end = sample.phases[next]?.eventIndex ?? sample.events.length
    const events = sample.events.slice(start, end)
    return { name, nativeListStarts: events.filter(e => e.type === 'fetch-start' && e.path.endsWith('/matches')).length, nativeTableStarts: events.filter(e => e.type === 'fetch-start' && e.path.endsWith('/match-table')).length, list: counts(requests.filter(r => r.path.endsWith('/matches'))), table: counts(requests.filter(r => r.path.endsWith('/match-table'))),
      rounds: counts(requests.filter(r => r.path.endsWith('/rounds'))), auth: counts(requests.filter(r => r.path.endsWith('/session'))),
      streamStarts: requests.filter(r => r.path.endsWith('/live')).length,
      dispatches: events.filter(e => e.type === 'dispatch-start').map(e => ({ signal: e.signal, source: e.source, id: e.dispatch,
        listStarts: events.filter(f => f.type === 'fetch-start' && f.dispatch === e.dispatch && f.path.endsWith('/matches')).length,
        tableStarts: events.filter(f => f.type === 'fetch-start' && f.dispatch === e.dispatch && f.path.endsWith('/match-table')).length })),
    }
  }), held: sample.server.filter(r => r.held).map(r => ({ path: r.path, phase: r.phase, epoch: r.epoch, expectedCompletion: [2,4].includes(r.epoch), aborted: r.aborted, closedBeforeRelease: r.closeMs < r.releaseMs, finished: r.finishMs !== undefined })),
  overflow: sample.overflow, errors: sample.errors, serverErrors: sample.serverErrors, pendingNonSse: sample.pendingNonSse, finalEpoch: sample.finalEpoch,
}))
if (!samples.length || samples.some(s => s.errors.length || s.serverErrors.length || s.overflow || s.pendingNonSse || s.finalEpoch !== 10 || !s.expectedDenials.length || s.held.some(r => r.expectedCompletion ? (!r.finished || r.aborted) : (!r.aborted || !r.closedBeforeRelease || r.finished)))) throw new Error('Incomplete/failed sample')
const { samples: rawSamples, ...meta } = trace
writeFileSync(resolve(out, 'summary.json'), JSON.stringify({ ...meta, sampleCount: samples.length, samples }, null, 2) + '\n')
const header = 'route,width,repeat,initial,phase,listStarts,listCompleted,listAborted,tableStarts,tableCompleted,tableAborted,roundStarts,authStarts,streamStarts,nativeListStarts,nativeTableStarts'
writeFileSync(resolve(out, 'samples.csv'), header + '\n' + samples.flatMap(s => s.phases.map(p => [s.route,s.width,s.repeat,s.initial,p.name,p.list.starts,p.list.completed,p.list.aborted,p.table.starts,p.table.completed,p.table.aborted,p.rounds.starts,p.auth.starts,p.streamStarts,p.nativeListStarts,p.nativeTableStarts].join(','))).join('\n') + '\n')
writeFileSync(resolve(out, 'traces.jsonl'), rawSamples.map(s => JSON.stringify({ ...s, requests: s.requests.filter(r => r.path.startsWith('/api/')), server: s.server.filter(r => r.path.startsWith('/api/')) })).join('\n') + '\n')
console.log(`${samples.length} complete samples retained`)
