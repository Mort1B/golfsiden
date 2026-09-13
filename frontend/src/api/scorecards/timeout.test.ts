// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest'
import { scoreRequest } from './timeout'

afterEach(() => vi.useRealTimers())
it('aborts a hanging score read before the delivery lease expires and cleans timers', async () => {
  vi.useFakeTimers()
  const pending = scoreRequest(signal => new Promise<void>((_resolve, reject) => signal.addEventListener('abort', () => reject(new DOMException('Timed out', 'AbortError')), { once: true })))
  const rejected = expect(pending).rejects.toMatchObject({ name: 'AbortError' })
  await vi.advanceTimersByTimeAsync(12_001)
  await rejected
  expect(vi.getTimerCount()).toBe(0)
})
it('propagates session/query cancellation and cleans the timeout', async () => {
  vi.useFakeTimers()
  const parent = new AbortController()
  const pending = scoreRequest(signal => new Promise<void>((_resolve, reject) => signal.addEventListener('abort', () => reject(new DOMException('Cancelled', 'AbortError')), { once: true })), parent.signal)
  const rejected = expect(pending).rejects.toMatchObject({ name: 'AbortError' })
  parent.abort(); await rejected
  expect(vi.getTimerCount()).toBe(0)
})
