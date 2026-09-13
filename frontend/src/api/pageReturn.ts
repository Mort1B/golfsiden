// A restored page need not remount React or trigger a window-focus event.
export function subscribePageReturn(onReturn: () => void): () => void {
  let active = true
  let queued = false
  const resume = () => {
    if (document.visibilityState === 'hidden' || queued) return
    queued = true
    queueMicrotask(() => {
      queued = false
      if (active) onReturn()
    })
  }
  const restored = (event: PageTransitionEvent) => { if (event.persisted) resume() }
  document.addEventListener('visibilitychange', resume)
  window.addEventListener('pageshow', restored)
  window.addEventListener('online', resume)
  return () => {
    active = false
    document.removeEventListener('visibilitychange', resume)
    window.removeEventListener('pageshow', restored)
    window.removeEventListener('online', resume)
  }
}
