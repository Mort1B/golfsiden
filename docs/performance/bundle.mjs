import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { gzipSync, brotliCompressSync } from 'node:zlib'
import { createHash } from 'node:crypto'
const root = fileURLToPath(new URL('../../', import.meta.url))
const require = createRequire(resolve(root, 'frontend/package.json'))
const { build } = await import(require.resolve('vite'))
const modules = []
const built = await build({ root: resolve(root, 'frontend'), build: { write: false }, plugins: [{
  name: 'baseline-module-measurement',
  generateBundle(_options, bundle) {
    for (const chunk of Object.values(bundle)) if (chunk.type === 'chunk') {
      for (const [path, module] of Object.entries(chunk.modules)) modules.push({ path: path.replace(root, ''), renderedBytes: module.renderedLength })
    }
  },
}] })
for (const output of (Array.isArray(built) ? built : [built]).flatMap(result => result.output)) {
  const fresh = Buffer.from(output.type === 'chunk' ? output.code : output.source)
  if (!fresh.equals(readFileSync(resolve(root, 'frontend/dist', output.fileName)))) throw new Error(`Stale build: ${output.fileName}; run npm --prefix frontend run build`)
}
const groups = {}
for (const module of modules) {
  const path = module.path
  const group = path.includes('/node_modules/') ? path.split('/node_modules/').at(-1).split('/').slice(0, path.split('/node_modules/').at(-1).startsWith('@') ? 2 : 1).join('/')
    : path.startsWith('frontend/src/features/') ? path.split('/').slice(0, 4).join('/')
    : path.startsWith('frontend/src/') ? path.split('/').slice(0, 3).join('/') : 'other'
  groups[group] = (groups[group] ?? 0) + module.renderedBytes
}
const assets = readdirSync(resolve(root, 'frontend/dist/assets')).map(name => {
  const data = readFileSync(resolve(root, 'frontend/dist/assets', name))
  return { name, sha256: createHash('sha256').update(data).digest('hex'), bytes: data.length, gzipBytes: gzipSync(data).length, brotliBytes: brotliCompressSync(data).length }
})
writeFileSync(process.argv[2] ?? '/tmp/golf-baseline-bundle.json', JSON.stringify({ assets,
  note: 'Rollup renderedLength attribution is before final minification; gzip/Brotli sizes are computed, not production transfer measurements.',
  groups: Object.entries(groups).sort((a, b) => b[1] - a[1]),
  largestModules: modules.sort((a, b) => b.renderedBytes - a.renderedBytes).slice(0, 25),
}, null, 2) + '\n')
