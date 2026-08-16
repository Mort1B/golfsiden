import { describe, expect, it } from 'vitest'
import { ApiHttpError } from '../../api/http'
import { joinErrorMessage, previewFailure } from './joinMessages'

describe('invitation error presentation', () => {
  it.each([
    ['invitation_invalid', 'invalid'],
    ['invitation_expired', 'expired'],
    ['invitation_revoked', 'revoked'],
    ['invitation_exhausted', 'exhausted'],
    ['tournament_not_joinable', 'closed'],
  ] as const)('maps %s to the finite preview state %s', (code, expected) => {
    expect(previewFailure(new ApiHttpError(409, code, 'server message'))).toBe(expected)
  })

  it('keeps network and unexpected server errors retryable', () => {
    expect(previewFailure(new TypeError('Failed to fetch'))).toBe('retryable')
    expect(previewFailure(new ApiHttpError(500, 'internal_error', 'server message'))).toBe('retryable')
  })

  it('uses stable product guidance for an unlinked account', () => {
    expect(joinErrorMessage(new ApiHttpError(409, 'account_player_required', 'server message')))
      .toContain('ikke koblet til en spillerprofil')
  })
})
