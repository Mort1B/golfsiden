import { describe, expect, it } from 'vitest'
import { ApiHttpError } from '../../api/http'
import {
  configurationFailure,
  createManualDraft,
  resizeManualDraft,
  validateCatalogSearch,
  validateManualDraft,
} from './courseConfiguration'

function validDraft() {
  return {
    ...createManualDraft(2), courseName: '  Østlandet Golfklubb  ', location: '   ', teeName: 'Hvit',
    courseRating: '71,2', slopeRating: '125',
    holes: [{ par: '4', strokeIndex: '2', distance: '401' }, { par: '3', strokeIndex: '1', distance: ' ' }],
  }
}

describe('manual course validation', () => {
  it('normalizes comma decimals and blank optional values without inventing distance', () => {
    const result = validateManualDraft(validDraft())
    expect(result.ok).toBe(true)
    if (!result.ok) throw new Error('Expected valid input')
    expect(result.value).toEqual({
      source: 'manual', course_name: 'Østlandet Golfklubb', location: null,
      tee: { category: 'male', name: 'Hvit', course_rating: 71.2, slope_rating: 125,
        holes: [{ par: 4, stroke_index: 2, distance: 401 }, { par: 3, stroke_index: 1, distance: null }] },
    })
  })

  it('rejects UTF-8 byte overflow, extra decimals, invalid ranges, and missing values', () => {
    expect(validateManualDraft({ ...validDraft(), courseName: 'å'.repeat(151) }).ok).toBe(false)
    expect(validateManualDraft({ ...validDraft(), courseRating: '71,25' }).ok).toBe(false)
    expect(validateManualDraft({ ...validDraft(), slopeRating: '54' }).ok).toBe(false)
    expect(validateManualDraft({ ...validDraft(), holes: [{ par: '', strokeIndex: '2', distance: '' }, { par: '3', strokeIndex: '1', distance: '' }] }).ok).toBe(false)
  })

  it('marks duplicate and incomplete stroke-index permutations and invalid optional distances', () => {
    const duplicate = validateManualDraft({ ...validDraft(), holes: [{ par: '4', strokeIndex: '1', distance: '' }, { par: '3', strokeIndex: '1', distance: '0' }] })
    expect(duplicate.ok).toBe(false)
    if (duplicate.ok) throw new Error('Expected invalid input')
    expect(duplicate.holeErrors).toEqual({
      0: { strokeIndex: 'Slagindeksen er brukt flere ganger.' },
      1: {
        strokeIndex: 'Slagindeksen er brukt flere ganger.',
        distance: 'Avstand må være et positivt heltall i yards.',
      },
    })
    expect(duplicate.firstInvalid).toBe('holes.0.strokeIndex')
  })

  it('reports each invalid field separately and supports 1–36 manual holes', () => {
    const original = createManualDraft(18)
    original.holes[2] = { par: '5', strokeIndex: '3', distance: '333' }
    original.holes[17] = { par: '4', strokeIndex: '18', distance: '444' }
    const resized = resizeManualDraft(resizeManualDraft(original, 3), 36)
    expect(resized.holes[2]).toEqual({ par: '5', strokeIndex: '3', distance: '333' })
    expect(resized.holes[17]).toEqual({ par: '4', strokeIndex: '18', distance: '444' })
    expect(resized.holes).toHaveLength(36)
    const invalid = validateManualDraft({ ...resized, courseName: '', holeCount: '37' })
    expect(invalid.ok).toBe(false)
    if (invalid.ok) throw new Error('Expected invalid input')
    expect(invalid.fieldErrors).toMatchObject({ holeCount: expect.any(String), courseName: expect.any(String) })
    expect(invalid.firstInvalid).toBe('holeCount')
  })
})

describe('catalog search', () => {
  it('normalizes valid searches and locally rejects one character and UTF-8 byte overflow', () => {
    expect(validateCatalogSearch('  DRØBAK GK ')).toEqual({ ok: true, normalized: 'drøbak gk' })
    expect(validateCatalogSearch('d')).toMatchObject({ ok: false, normalized: 'd' })
    expect(validateCatalogSearch('ø')).toEqual({ ok: true, normalized: 'ø' })
    expect(validateCatalogSearch('å'.repeat(41))).toMatchObject({ ok: false })
    expect(validateCatalogSearch('   ')).toEqual({ ok: true, normalized: '' })
  })
})

describe('configuration failures', () => {
  it.each([
    ['round_configuration_stale', 'stale'], ['round_not_draft', 'not-draft'],
    ['course_provider_tee_stale', 'tee-stale'], ['forbidden', 'access'],
  ] as const)('classifies %s', (code, expected) => {
    expect(configurationFailure(new ApiHttpError(code === 'forbidden' ? 403 : 409, code, 'failed'))).toBe(expected)
  })
})
