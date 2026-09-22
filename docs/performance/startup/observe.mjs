// Observation only: preserve native fetch/EventSource behavior and record metadata.
export function installObserver(origin) {
  const events = []
  window.startupTrace = events
  const log = (type, extra = {}) => events.push({ type, atMs: Date.now() - origin, ...extra })
  const nativeFetch = window.fetch
  let serial = 0
  window.fetch = function (input, init) {
    const id = ++serial, path = new URL(input instanceof Request ? input.url : String(input), location.href).pathname
    const signal = init?.signal ?? (input instanceof Request ? input.signal : undefined)
    log('fetch-start', { id, path, hasSignal: !!signal })
    signal?.addEventListener('abort', () => log('fetch-abort', { id, path }), { once: true })
    return nativeFetch.call(this, input, init).then(response => {
      log('fetch-headers', { id, path, status: response.status }); return response
    }, error => { log('fetch-reject', { id, path, name: error.name }); throw error })
  }
  const NativeSource = window.EventSource
  window.EventSource = class extends NativeSource {
    constructor(url, options) {
      super(url, options)
      for (const type of ['open', 'error', 'match']) this.addEventListener(type, () => log(`sse-${type}`))
    }
  }
  let previous = ''
  const observe = () => {
    const cards = [...document.querySelectorAll('.match-list article h3')], rows = [...document.querySelectorAll('.match-table li strong')]
    const epochs = [...new Set([...cards, ...rows].flatMap(el => [...el.textContent.matchAll(/\[epoch (\d+)\]/g)].map(m => Number(m[1]))))]
    const content = { cards: cards.length, rows: rows.length, epochs }
    const value = JSON.stringify(content)
    if (value !== previous) { previous = value; log('content', content) }
  }
  new MutationObserver(observe).observe(document, { childList: true, subtree: true, characterData: true })
}
