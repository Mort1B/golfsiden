import { describe, expect, it } from 'vitest'
import { parseInvitationSecret } from './secret'

const token = `${'aB9_-'.repeat(8)}abc`

describe('invitation fragment parsing', () => {
  it('accepts only one exact 43-character URL-safe fragment token', () => {
    expect(parseInvitationSecret(`#token=${token}`)).toEqual({ status: 'valid', token })
  })

  it.each([
    ['', 'missing'],
    ['#', 'missing'],
    ['#token=short', 'malformed'],
    [`#token=${token}&other=value`, 'malformed'],
    [`#other=value&token=${token}`, 'malformed'],
    [`?token=${token}`, 'malformed'],
    [`#token=${token}=`, 'malformed'],
  ])('classifies %s without returning a secret', (fragment, status) => {
    expect(parseInvitationSecret(fragment)).toEqual({ status })
  })
})
