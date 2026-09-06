import { expect, it } from 'vitest'
import { decodeCoursePresets, presetKey } from './coursePresets'
import { presetResponse, suppliedCourses } from './coursePresets.fixture'
it('decodes all exact supplied facts into independent manual requests', () => {
  const result = decodeCoursePresets(presetResponse)
  result.forEach(({ id, selection }, i) => {
    const c = suppliedCourses[i]
    if (!c) throw new Error('Missing expected course')
    expect(id).toBe(c.id)
    expect(selection).toEqual({ source: 'manual', course_name: c.n, location: null, tee: { category: 'male', name: 'Red Tees', course_rating: c.r, slope_rating: c.s, holes: c.p.map((par, j) => ({ par, stroke_index: c.i[j], distance: null })) } })
    expect(selection.tee.holes.reduce((sum, h) => sum + h.par, 0)).toBe(72)
  })
  expect(presetKey('user', 'trip')).toEqual(['private-workspace', 'user', 'course-presets', 'trip'])
  expect(decodeCoursePresets([])).toEqual([])
})
it.each(['duplicate-id', 'duplicate-index', 'unordered', 'missing-hole', 'wrong-category', 'bad-rating', 'bad-slope', 'missing-distance'])('rejects malformed %s', (kind) => {
  const v = JSON.parse(JSON.stringify(presetResponse))
  if (kind === 'duplicate-id') v[1].id = v[0].id
  if (kind === 'duplicate-index') v[0].tee.holes[0].stroke_index = v[0].tee.holes[1].stroke_index
  if (kind === 'unordered') v[0].tee.holes.reverse()
  if (kind === 'missing-hole') v[0].tee.holes.pop()
  if (kind === 'wrong-category') v[0].tee.category = 'red'
  if (kind === 'bad-rating') v[0].tee.course_rating = 72.55
  if (kind === 'bad-slope') v[0].tee.slope_rating = 54
  if (kind === 'missing-distance') delete v[0].tee.holes[0].distance
  expect(() => decodeCoursePresets(v)).toThrow()
})
