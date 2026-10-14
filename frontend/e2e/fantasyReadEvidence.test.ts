import { describe, expect, it } from 'vitest'
import { classifyFantasyRead } from './fantasyReadEvidence'

const paths = new Set(['/fantasy/results', '/fantasy/rounds/one/results'])
const base = { method: 'GET', path: '/fantasy/results', status: 409, code: 'fantasy_conflict', bodyReadable: true, failure: null }
const classify = (patch: Partial<Omit<typeof base, 'failure'>> & { failure?: string | null } = {}) =>
  classifyFantasyRead({ ...base, ...patch }, paths, '/fantasy/rounds/three')
describe('Fantasy browser error classification', () => {
  it('accepts only the expected status, endpoint, method and decoded code', () => {
    expect(classify()).toBe('conflict')
    expect(classify({ path: '/fantasy/rounds/three', status: 503, code: 'unavailable' })).toBe('unavailable')
    for (const patch of [{ code: 'fantasy_revision_conflict' }, { method: 'PUT' }, { path: '/elsewhere' }, { status: 500 }]) {
      expect(classify(patch)).toBe('unexpected')
    }
    expect(classify({ path: '/fantasy/rounds/three', status: 503, code: 'wrong' })).toBe('unexpected')
  })
  it('requires positive cancellation evidence for an unread expected read', () => {
    expect(classify({ bodyReadable: false })).toBe('unexpected')
    expect(classify({ bodyReadable: false, failure: 'net::ERR_CONNECTION_RESET' })).toBe('unexpected')
    expect(classify({ bodyReadable: false, failure: 'net::ERR_ABORTED' })).toBe('cancelled')
    expect(classify({ method: 'PUT', bodyReadable: false, failure: 'net::ERR_ABORTED' })).toBe('unexpected')
    expect(classify({ code: 'wrong', failure: 'net::ERR_ABORTED' })).toBe('unexpected')
  })
})
