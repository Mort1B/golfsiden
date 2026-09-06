import { expect, it } from 'vitest'
import { passwordValidationMessage } from './password'

it.each(['a', 'ø', '🏌'])('measures %s passwords in UTF-8 bytes without trimming', (unit) => {
  const size = new TextEncoder().encode(unit).length
  expect(passwordValidationMessage(unit.repeat(12 / size))).toBeNull()
  expect(passwordValidationMessage(unit.repeat(128 / size))).toBeNull()
  expect(passwordValidationMessage(unit.repeat(12 / size - 1))).toContain('for kort')
  expect(passwordValidationMessage(unit.repeat(128 / size + 1))).toContain('for langt')
})
it('preserves spaces and distinguishes exact ASCII boundaries', () => {
  expect(passwordValidationMessage(' '.repeat(12))).toBeNull()
  expect(passwordValidationMessage('a'.repeat(11))).toContain('minst 12 byte')
  expect(passwordValidationMessage('a'.repeat(129))).toContain('128 byte')
})
