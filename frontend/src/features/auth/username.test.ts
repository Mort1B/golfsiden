import { describe, expect, it } from 'vitest'
import { USERNAME_HTML_PATTERN } from './username'

const htmlPattern = new RegExp(`^(?:${USERNAME_HTML_PATTERN})$`, 'v')

describe('USERNAME_HTML_PATTERN', () => {
  it('is compatible with HTML pattern v-mode compilation', () => {
    expect(USERNAME_HTML_PATTERN).toBe('[A-Za-z0-9_\\-]{3,32}')
    expect(() => new RegExp(USERNAME_HTML_PATTERN, 'v')).not.toThrow()
  })

  it.each([
    'abc',
    'A_1',
    'golf-spiller_2026',
    'a'.repeat(32),
  ])('accepts %s', (username) => {
    expect(htmlPattern.test(username)).toBe(true)
  })

  it.each([
    'ab',
    'a'.repeat(33),
    'golf spiller',
    'golf.spiller',
    'spilleræ',
  ])('rejects %s', (username) => {
    expect(htmlPattern.test(username)).toBe(false)
  })
})
