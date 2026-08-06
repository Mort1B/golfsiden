import { describe, expect, it } from 'vitest'
import { safeReturnTo, signInTarget } from './navigation'

describe('authentication navigation', () => {
  it('preserves the complete protected score URL', () => {
    const target = signInTarget('/score', '?round=one&hole=7', '#entry')
    const returnTo = new URL(`https://local.test${target}`).searchParams.get('returnTo')
    expect(returnTo).toBe('/score?round=one&hole=7#entry')
    expect(safeReturnTo(returnTo)).toBe('/score?round=one&hole=7#entry')
  })

  it('rejects external and protocol-relative redirects', () => {
    expect(safeReturnTo('https://example.test')).toBe('/score')
    expect(safeReturnTo('//example.test')).toBe('/score')
    expect(safeReturnTo('/\\example.test')).toBe('/score')
    expect(safeReturnTo('///example.test')).toBe('/score')
  })
})
