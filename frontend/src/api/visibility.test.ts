import { describe, expect, it } from 'vitest'
import { decodeScoreVisibility, visibilityRefetchDelay } from './visibility'

describe('score visibility', () => {
  it('uses the server interval for a restricted projection', () => {
    const visibility = decodeScoreVisibility({
      mode: 'front_nine',
      observed_at: '2026-09-01T10:00:00Z',
      hidden_until: '2026-09-01T10:00:05Z',
    }, 'visibility', 'resultatdata')
    expect(visibilityRefetchDelay(visibility)).toBe(5_000)
  })

  it('does not schedule from browser time or an elapsed deadline', () => {
    expect(visibilityRefetchDelay({
      mode: 'full',
      observed_at: '2026-09-01T10:00:00Z',
      hidden_until: '2026-09-01T10:00:05Z',
    })).toBeNull()
    expect(visibilityRefetchDelay({
      mode: 'front_nine',
      observed_at: '2026-09-01T10:00:05Z',
      hidden_until: '2026-09-01T10:00:05Z',
    })).toBeNull()
  })

  it('rejects an unknown mode', () => {
    expect(() => decodeScoreVisibility({
      mode: 'back_nine',
      observed_at: '2026-09-01T10:00:00Z',
      hidden_until: null,
    }, 'visibility', 'resultatdata')).toThrow('visibility.mode')
  })
})
