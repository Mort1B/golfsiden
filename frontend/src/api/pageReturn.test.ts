// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest'
import { subscribePageReturn } from './pageReturn'

afterEach(() => vi.restoreAllMocks())
it('coalesces visible, restored and online returns and removes listeners on disposal', async () => {
  const visibility = vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden')
  const resume = vi.fn()
  const stop = subscribePageReturn(resume)
  document.dispatchEvent(new Event('visibilitychange'))
  window.dispatchEvent(new Event('online'))
  await Promise.resolve()
  expect(resume).not.toHaveBeenCalled()
  visibility.mockReturnValue('visible')
  document.dispatchEvent(new Event('visibilitychange'))
  window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true }))
  window.dispatchEvent(new Event('online'))
  await Promise.resolve()
  expect(resume).toHaveBeenCalledOnce()
  window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: false }))
  await Promise.resolve()
  expect(resume).toHaveBeenCalledOnce()
  document.dispatchEvent(new Event('visibilitychange'))
  stop()
  await Promise.resolve()
  window.dispatchEvent(new Event('online'))
  await Promise.resolve()
  expect(resume).toHaveBeenCalledOnce()
})
