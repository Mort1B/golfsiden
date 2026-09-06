import { decodeArray, decodeInteger, decodeNumber, decodeObject, decodeString, decodeUuid, invalidData } from './decoder'
import { requestDecoded } from './http'
import { privateWorkspaceKeys } from './privateWorkspace'
import type { ManualCourseSelection } from './courses'

export interface CoursePreset { id: string; selection: ManualCourseSelection }
export const presetKey = (userId: string, tournamentId: string) => [...privateWorkspaceKeys.user(userId), 'course-presets', tournamentId] as const

export function decodeCoursePresets(value: unknown): CoursePreset[] {
  const presets = decodeArray(value, 'presets', (value, path): CoursePreset => {
    const item = decodeObject(value, path)
    const tee = decodeObject(item.tee, `${path}.tee`)
    if (tee.category !== 'male' && tee.category !== 'female') return invalidData('banedata', `${path}.tee.category`)
    const holes = decodeArray(tee.holes, `${path}.tee.holes`, (value, holePath) => {
      const hole = decodeObject(value, holePath)
      return { number: decodeInteger(hole.number, `${holePath}.number`, 1, 36), par: decodeInteger(hole.par, `${holePath}.par`, 2, 7), stroke_index: decodeInteger(hole.stroke_index, `${holePath}.stroke_index`, 1, 36), distance: hole.distance === null ? null : decodeInteger(hole.distance, `${holePath}.distance`, 1, 32767) }
    })
    if (!holes.length || holes.length > 36 || holes.some((hole, i) => hole.number !== i + 1 || hole.stroke_index > holes.length) || new Set(holes.map((hole) => hole.stroke_index)).size !== holes.length) return invalidData('banedata', `${path}.tee.holes`)
    const courseName = decodeString(item.course_name, `${path}.course_name`)
    const name = decodeString(tee.name, `${path}.tee.name`)
    const rating = decodeNumber(tee.course_rating, `${path}.tee.course_rating`, 1, 100)
    if (!courseName.trim() || courseName.length > 300 || !name.trim() || name.length > 100 || Math.abs(rating * 10 - Math.round(rating * 10)) > 1e-8) return invalidData('banedata', path)
    return { id: decodeUuid(item.id, `${path}.id`), selection: {
      source: 'manual', course_name: courseName, location: item.location === null ? null : decodeString(item.location, `${path}.location`),
      tee: { category: tee.category, name, course_rating: rating, slope_rating: decodeInteger(tee.slope_rating, `${path}.tee.slope_rating`, 55, 155), holes: holes.map(({ par, stroke_index, distance }) => ({ par, stroke_index, distance })) },
    } }
  })
  if (new Set(presets.map((preset) => preset.id)).size !== presets.length) return invalidData('banedata', 'presets.ids')
  return presets
}
export const presetApi = { list: (tournamentId: string) => requestDecoded(`/api/tournaments/${tournamentId}/course-presets`, decodeCoursePresets) }
