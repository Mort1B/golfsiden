import { afterEach, describe, expect, it, vi } from 'vitest'
import { courseApi, courseKeys, decodeCourseCatalog, decodeProviderCourse } from './courses'

const providerId = '0zm1pe1a'
const roundId = '00000000-0000-0000-0000-000000000001'
const tournamentId = '00000000-0000-0000-0000-000000000002'

afterEach(() => vi.unstubAllGlobals())

describe('course API decoders', () => {
  it('decodes the exact catalog envelope and rejects an unknown status', () => {
    const response = { courses: [{
      display_name: 'Langt banenavn', country: 'Norway', provider: 'golf_course_api',
      provider_course_id: providerId, provider_status: 'usable', provider_status_detail: 'Klar',
    }] }
    expect(decodeCourseCatalog(response)).toEqual(response.courses)
    expect(() => decodeCourseCatalog({ courses: [{ ...response.courses[0], provider_status: 'ready' }] })).toThrow('provider_status')
  })

  it('decodes complete ordered tee facts and rejects stale identities or incomplete indexes', () => {
    const response = {
      provider: 'golf_course_api', provider_course_id: providerId, club_name: 'Klubben', course_name: 'Banen',
      scorecard_url: null, location: { address: null, city: 'Oslo', state: null, country: 'Norway' },
      tees: [{ category: 'male', name: 'Hvit', course_rating: 71.2, slope_rating: 125,
        total_yards: 700, total_meters: 640, number_of_holes: 2, par_total: 7,
        holes: [{ number: 1, par: 4, yardage: 400, stroke_index: 2 }, { number: 2, par: 3, yardage: 300, stroke_index: 1 }] }],
    }
    expect(decodeProviderCourse(response, providerId).tees[0]?.holes).toHaveLength(2)
    expect(() => decodeProviderCourse(response, 'other')).toThrow('identity')
    const tee = response.tees[0]
    if (!tee) throw new Error('Expected fixture tee')
    expect(() => decodeProviderCourse({ ...response, tees: [{ ...tee, holes: tee.holes.map((hole) => ({ ...hole, stroke_index: 1 })) }] }, providerId)).toThrow('holes')
    expect(() => decodeProviderCourse({
      ...response,
      tees: [tee, { ...tee, name: ' Hvit ' }],
    }, providerId)).toThrow('selector')
  })

  it('roots private keys by user and sends the visible optimistic token with CSRF', async () => {
    const response = {
      id: roundId, tournament_id: tournamentId, round_number: 1, name: 'Runde 1', round_date: '2026-09-01',
      course_id: '00000000-0000-0000-0000-000000000003', course_name: 'Banen',
      tee_id: '00000000-0000-0000-0000-000000000004', tee_name: 'Hvit', number_of_holes: 1,
      status: 'draft', handicap_enabled: true, handicap_allowance_percent: 100,
      scoring_format: 'individual_stroke_play', created_at: '2026-08-23T12:00:00Z', updated_at: '2026-08-23T12:01:00Z',
    }
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      void input
      void init
      return new Response(JSON.stringify(response), { status: 200 })
    })
    vi.stubGlobal('fetch', fetchMock)
    const selection = { source: 'golf_course_api' as const, provider_course_id: providerId, tee: { category: 'male' as const, name: 'Hvit' } }

    await courseApi.configure(roundId, tournamentId, '2026-08-23T12:00:00Z', selection, 'csrf')

    expect(courseKeys.catalog('user-one', tournamentId, 'oslo').slice(0, 2)).toEqual(['private-workspace', 'user-one'])
    expect(courseKeys.provider('user-two', tournamentId, providerId).slice(0, 2)).toEqual(['private-workspace', 'user-two'])
    expect(fetchMock.mock.calls[0]?.[0]).toBe(`/api/rounds/${roundId}/course-configuration`)
    expect(fetchMock.mock.calls[0]?.[1]).toMatchObject({
      method: 'PUT', headers: { 'content-type': 'application/json', 'x-csrf-token': 'csrf' },
      body: JSON.stringify({ expected_round_updated_at: '2026-08-23T12:00:00Z', selection }),
    })

    fetchMock.mockResolvedValueOnce(new Response(JSON.stringify({
      ...response, id: '00000000-0000-0000-0000-000000000099',
    }), { status: 200 }))
    await expect(courseApi.configure(
      roundId, tournamentId, '2026-08-23T12:00:00Z', selection, 'csrf',
    )).rejects.toThrow('identity')

    fetchMock.mockResolvedValueOnce(new Response(JSON.stringify({
      ...response, tournament_id: '00000000-0000-0000-0000-000000000099',
    }), { status: 200 }))
    await expect(courseApi.configure(
      roundId, tournamentId, '2026-08-23T12:00:00Z', selection, 'csrf',
    )).rejects.toThrow('tournament_id')
  })

  it('encodes a normalized catalog search as q and keeps it in the private key', async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      void input
      return new Response(JSON.stringify({ courses: [] }), { status: 200 })
    })
    vi.stubGlobal('fetch', fetchMock)

    await courseApi.catalog(tournamentId, 'drøbak gk')

    expect(courseKeys.catalog('user-one', tournamentId, 'drøbak gk')).toEqual([
      'private-workspace', 'user-one', 'course-catalog', tournamentId, 'drøbak gk',
    ])
    expect(fetchMock.mock.calls[0]?.[0]).toBe(`/api/tournaments/${tournamentId}/course-catalog?q=dr%C3%B8bak+gk`)
  })
})
