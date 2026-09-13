// A request timeout is shorter than the device's cross-tab lease.
export async function scoreRequest<T>(action: (signal: AbortSignal) => Promise<T>, parent?: AbortSignal): Promise<T> {
  const controller = new AbortController()
  const abort = () => controller.abort()
  if (parent?.aborted) controller.abort()
  parent?.addEventListener('abort', abort, { once: true })
  const timer = window.setTimeout(abort, 12_000)
  try { return await action(controller.signal) }
  finally { window.clearTimeout(timer); parent?.removeEventListener('abort', abort) }
}
