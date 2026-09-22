// Native calls remain unchanged; dispatch tags expose synchronous request ownership.
export function installObserver(origin) {
  const events = [], log = (type, extra = {}) => events.push({ type, atMs: Date.now() - origin, ...extra })
  window.subscriberTrace = events
  let serial = 0, sourceSerial = 0, dispatchSerial = 0, dispatch = null
  const nativeFetch = window.fetch
  window.fetch = function (input, init) {
    const id = ++serial, path = new URL(input instanceof Request ? input.url : String(input), location.href).pathname
    const signal = init?.signal ?? (input instanceof Request ? input.signal : undefined)
    log('fetch-start', { id, path, hasSignal: !!signal, dispatch })
    signal?.addEventListener('abort', () => log('fetch-abort', { id, path, dispatch }), { once: true })
    return nativeFetch.call(this, input, init).then(response => {
      log('fetch-headers', { id, path, status: response.status }); return response
    }, error => { log('fetch-reject', { id, path, name: error.name }); throw error })
  }
  const NativeSource = window.EventSource
  window.EventSource = class extends NativeSource {
    constructor(url, options) { super(url, options); this.traceId = ++sourceSerial; log('source-create', { source: this.traceId }) }
    addEventListener(type, listener, options) {
      // Production subscription uses function listeners; retain object-listener compatibility.
      return super.addEventListener(type, event => {
        const previous = dispatch
        dispatch = ++dispatchSerial
        log('dispatch-start', { signal: type, source: this.traceId, dispatch })
        try { if (typeof listener === 'function') listener.call(this, event); else listener.handleEvent(event) }
        finally { log('dispatch-end', { signal: type, source: this.traceId, dispatch }); dispatch = previous }
      }, options)
    }
    close() { log('source-close', { source: this.traceId }); return super.close() }
  }
  let previous = ''
  new MutationObserver(() => {
    const cards = [...document.querySelectorAll('.match-list article h3')], rows = [...document.querySelectorAll('.match-table li strong')]
    const epochs = [...new Set([...cards, ...rows].flatMap(el => [...el.textContent.matchAll(/\[epoch (\d+)\]/g)].map(m => Number(m[1]))))]
    const value = { cards: cards.length, rows: rows.length, epochs }
    if (JSON.stringify(value) !== previous) { previous = JSON.stringify(value); log('content', value) }
  }).observe(document, { childList: true, subtree: true, characterData: true })
}
