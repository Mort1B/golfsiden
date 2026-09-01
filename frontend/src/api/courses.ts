import {
  decodeArray,
  decodeInteger,
  decodeNumber,
  decodeObject,
  decodeString,
  invalidData,
} from './decoder'
import { jsonRequest, requestDecoded } from './http'
import { privateWorkspaceKeys } from './privateWorkspace'
import { decodeExpectedRound } from './tournaments'
import type { Round } from './types'

export type TeeCategory = 'female' | 'male'
export type ProviderStatus = 'usable' | 'incomplete' | 'missing'

export interface CourseCatalogItem {
  display_name: string
  country: string
  provider: 'golf_course_api'
  provider_course_id: string | null
  provider_status: ProviderStatus
  provider_status_detail: string
}

export interface ProviderHole {
  number: number
  par: number
  yardage: number
  stroke_index: number
}

export interface ProviderTee {
  category: TeeCategory
  name: string
  course_rating: number
  slope_rating: number
  total_yards: number
  total_meters: number
  number_of_holes: number
  par_total: number
  holes: ProviderHole[]
}

export interface ProviderCourseDetail {
  provider: 'golf_course_api'
  provider_course_id: string
  club_name: string
  course_name: string
  scorecard_url: string | null
  location: { address: string | null; city: string | null; state: string | null; country: string | null }
  tees: ProviderTee[]
}

export interface ManualHoleInput { par: number; stroke_index: number; distance: number | null }
export interface ManualCourseSelection {
  source: 'manual'
  course_name: string
  location: string | null
  tee: { category: TeeCategory; name: string; course_rating: number; slope_rating: number; holes: ManualHoleInput[] }
}
export interface ProviderCourseSelection {
  source: 'golf_course_api'
  provider_course_id: string
  tee: { category: TeeCategory; name: string }
}
export type CourseSelection = ManualCourseSelection | ProviderCourseSelection

export const courseKeys = {
  catalogRoot: (userId: string) => [...privateWorkspaceKeys.user(userId), 'course-catalog'] as const,
  catalog: (userId: string, tournamentId: string, normalizedQuery: string) =>
    [...privateWorkspaceKeys.user(userId), 'course-catalog', tournamentId, normalizedQuery] as const,
  providerRoot: (userId: string) => [...privateWorkspaceKeys.user(userId), 'course-provider'] as const,
  provider: (userId: string, tournamentId: string, providerCourseId: string) =>
    [...privateWorkspaceKeys.user(userId), 'course-provider', tournamentId, providerCourseId] as const,
}

function literal<T extends string>(value: unknown, expected: readonly T[], path: string): T {
  if (typeof value === 'string' && expected.includes(value as T)) return value as T
  return invalidData('banedata', path)
}

function nullableString(value: unknown, path: string): string | null {
  return value === null ? null : decodeString(value, path, 'banedata')
}

function boundedString(value: unknown, path: string, maximumBytes: number, allowEmpty = false): string {
  const decoded = decodeString(value, path, 'banedata')
  if ((!allowEmpty && !decoded.trim()) || new TextEncoder().encode(decoded).length > maximumBytes || /\p{Cc}/u.test(decoded)) {
    invalidData('banedata', path)
  }
  return decoded
}

function providerId(value: unknown, path: string): string {
  const decoded = decodeString(value, path, 'banedata')
  if (!/^(?=.*[a-z])[0-9a-hjkmnp-tv-z]{8}$/.test(decoded)) invalidData('banedata', path)
  return decoded
}

function nullableBoundedString(value: unknown, path: string): string | null {
  return value === null ? null : boundedString(value, path, 500, true)
}

function decodeCatalogItem(value: unknown, path: string): CourseCatalogItem {
  const data = decodeObject(value, path, 'banedata')
  const decoded: CourseCatalogItem = {
    display_name: boundedString(data.display_name, `${path}.display_name`, 500),
    country: boundedString(data.country, `${path}.country`, 500),
    provider: literal(data.provider, ['golf_course_api'], `${path}.provider`),
    provider_course_id: data.provider_course_id === null
      ? null
      : providerId(data.provider_course_id, `${path}.provider_course_id`),
    provider_status: literal(data.provider_status, ['usable', 'incomplete', 'missing'], `${path}.provider_status`),
    provider_status_detail: boundedString(data.provider_status_detail, `${path}.provider_status_detail`, 500),
  }
  const hasId = decoded.provider_course_id !== null
  if ((decoded.provider_status === 'usable' && !hasId) || (decoded.provider_status === 'missing' && hasId)) {
    invalidData('banedata', `${path}.provider_course_id`)
  }
  return decoded
}

export function decodeCourseCatalog(value: unknown): CourseCatalogItem[] {
  const data = decodeObject(value, 'catalog', 'banedata')
  return decodeArray(data.courses, 'catalog.courses', decodeCatalogItem, 'banedata')
}

function decodeProviderHole(value: unknown, path: string): ProviderHole {
  const data = decodeObject(value, path, 'banedata')
  return {
    number: decodeInteger(data.number, `${path}.number`, 1, 36, 'banedata'),
    par: decodeInteger(data.par, `${path}.par`, 2, 7, 'banedata'),
    yardage: decodeInteger(data.yardage, `${path}.yardage`, 1, 2_000, 'banedata'),
    stroke_index: decodeInteger(data.stroke_index, `${path}.stroke_index`, 1, 36, 'banedata'),
  }
}

function decodeProviderTee(value: unknown, path: string): ProviderTee {
  const data = decodeObject(value, path, 'banedata')
  const holes = decodeArray(data.holes, `${path}.holes`, decodeProviderHole, 'banedata')
  const tee: ProviderTee = {
    category: literal(data.category, ['female', 'male'], `${path}.category`),
    name: boundedString(data.name, `${path}.name`, 100),
    course_rating: decodeNumber(data.course_rating, `${path}.course_rating`, 1, 100, 'banedata'),
    slope_rating: decodeInteger(data.slope_rating, `${path}.slope_rating`, 1, 200, 'banedata'),
    total_yards: decodeInteger(data.total_yards, `${path}.total_yards`, 0, undefined, 'banedata'),
    total_meters: decodeInteger(data.total_meters, `${path}.total_meters`, 0, undefined, 'banedata'),
    number_of_holes: decodeInteger(data.number_of_holes, `${path}.number_of_holes`, 1, 36, 'banedata'),
    par_total: decodeInteger(data.par_total, `${path}.par_total`, 2, 252, 'banedata'),
    holes,
  }
  const indexes = new Set(holes.map((hole) => hole.stroke_index))
  const ordered = holes.every((hole, index) => hole.number === index + 1)
  const parTotal = holes.reduce((total, hole) => total + hole.par, 0)
  const completeIndexes = holes.every((hole) => hole.stroke_index <= holes.length)
  if (holes.length !== tee.number_of_holes || indexes.size !== holes.length || !completeIndexes || !ordered || parTotal !== tee.par_total) {
    invalidData('banedata', `${path}.holes`)
  }
  return tee
}

export function decodeProviderCourse(value: unknown, expectedProviderCourseId: string): ProviderCourseDetail {
  const data = decodeObject(value, 'course', 'banedata')
  const location = decodeObject(data.location, 'course.location', 'banedata')
  const decoded: ProviderCourseDetail = {
    provider: literal(data.provider, ['golf_course_api'], 'course.provider'),
    provider_course_id: providerId(data.provider_course_id, 'course.provider_course_id'),
    club_name: boundedString(data.club_name, 'course.club_name', 300),
    course_name: boundedString(data.course_name, 'course.course_name', 300),
    scorecard_url: nullableString(data.scorecard_url, 'course.scorecard_url'),
    location: {
      address: nullableBoundedString(location.address, 'course.location.address'),
      city: nullableBoundedString(location.city, 'course.location.city'),
      state: nullableBoundedString(location.state, 'course.location.state'),
      country: nullableBoundedString(location.country, 'course.location.country'),
    },
    tees: decodeArray(data.tees, 'course.tees', decodeProviderTee, 'banedata'),
  }
  if (decoded.provider_course_id !== expectedProviderCourseId || decoded.tees.length === 0) {
    invalidData('banedata', 'course.identity')
  }
  const teeSelectors = new Set<string>()
  for (const [index, tee] of decoded.tees.entries()) {
    const selector = `${tee.category}:${tee.name.trim()}`
    if (teeSelectors.has(selector)) {
      invalidData('banedata', `course.tees[${index}].selector`)
    }
    teeSelectors.add(selector)
  }
  if (decoded.scorecard_url !== null) {
    let url: URL
    try { url = new URL(decoded.scorecard_url) } catch { return invalidData('banedata', 'course.scorecard_url') }
    if ((url.protocol !== 'http:' && url.protocol !== 'https:') || decoded.scorecard_url.length > 2_048) {
      invalidData('banedata', 'course.scorecard_url')
    }
  }
  return decoded
}

export const courseApi = {
  catalog: (tournamentId: string, query: string) => {
    const search = query ? `?${new URLSearchParams({ q: query })}` : ''
    return requestDecoded(`/api/tournaments/${tournamentId}/course-catalog${search}`, decodeCourseCatalog)
  },
  provider: (tournamentId: string, providerCourseId: string) => requestDecoded(
    `/api/tournaments/${tournamentId}/course-provider/courses/${encodeURIComponent(providerCourseId)}`,
    (value) => decodeProviderCourse(value, providerCourseId),
  ),
  configure: (
    roundId: string,
    tournamentId: string,
    expectedRoundUpdatedAt: string,
    selection: CourseSelection,
    csrfToken: string,
  ) =>
    requestDecoded(
      `/api/rounds/${roundId}/course-configuration`,
      (value): Round => {
        const round = decodeExpectedRound(value, roundId)
        if (round.tournament_id !== tournamentId) invalidData('rundedata', 'round.tournament_id identity')
        return round
      },
      jsonRequest('PUT', { expected_round_updated_at: expectedRoundUpdatedAt, selection }, csrfToken),
    ),
}
