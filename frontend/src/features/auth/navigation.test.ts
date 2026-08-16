import { describe, expect, it } from 'vitest'
import { safeReturnTo, signInTarget } from './navigation'

describe('authentication navigation', () => {
  it('preserves the complete protected score URL', () => {
    const target = signInTarget('/score', '?round=one&hole=7', '#entry')
    const returnTo = new URL(`https://local.test${target}`).searchParams.get('returnTo')
    expect(returnTo).toBe('/score?round=one&hole=7#entry')
    expect(safeReturnTo(returnTo)).toBe('/score?round=one&hole=7#entry')
  })

  it('preserves protected workspace detail and leaderboard URLs', () => {
    expect(safeReturnTo(new URL(`https://local.test${signInTarget('/tournaments/tour-one', '', '')}`)
      .searchParams.get('returnTo'))).toBe('/tournaments/tour-one')
    expect(safeReturnTo(new URL(`https://local.test${signInTarget('/leaderboard', '?scope=round', '')}`)
      .searchParams.get('returnTo'))).toBe('/leaderboard?scope=round')
  })

  it('rejects external and protocol-relative redirects', () => {
    expect(safeReturnTo('https://example.test')).toBe('/score')
    expect(safeReturnTo('//example.test')).toBe('/score')
    expect(safeReturnTo('/\\example.test')).toBe('/score')
    expect(safeReturnTo('///example.test')).toBe('/score')
  })
})
