import { readFileSync, writeFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
const hash = bytes => createHash('sha256').update(bytes).digest('hex')
const files = execFileSync('git', ['ls-files', 'frontend/src', 'backend/src'], { encoding: 'utf8' }).trim().split('\n')
writeFileSync(process.argv[2] ?? 'docs/performance/history-filter/source.json', JSON.stringify({
  productionSourceSha256: Object.fromEntries(files.map(file => [file, hash(readFileSync(file))])),
  productionDiffSha256: hash(execFileSync('git', ['diff', '--', 'frontend/src', 'backend/src'])),
}, null, 2) + '\n')
