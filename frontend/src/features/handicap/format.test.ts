import { describe, expect, it } from 'vitest'
import { formatHandicap, parseHandicap } from './format'

describe('Norwegian handicap handling', () => {
  it.each([
    ['14,4', 14.4],
    ['14.4', 14.4],
    [' -2,1 ', -2.1],
    ['54', 54],
    ['-10,0', -10],
    [',5', 0.5],
  ])('parses %s', (input, expected) => {
    expect(parseHandicap(input)).toEqual({ ok: true, value: expected })
  })

  it.each(['', '14,', '14.44', '1e1', '+4,0', '14,4.2', '55', '-10,1', 'NaN'])('rejects %s', (input) => {
    expect(parseHandicap(input).ok).toBe(false)
  })

  it('always displays one Norwegian decimal without grouping', () => {
    expect(formatHandicap(14.4)).toBe('14,4')
    expect(formatHandicap(4)).toBe('4,0')
  })
})
