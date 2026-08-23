import type { ManualCourseSelection, TeeCategory } from '../../api/courses'
import { ApiHttpError } from '../../api/http'

export interface ManualHoleDraft { par: string; strokeIndex: string; distance: string }
export interface ManualCourseDraft {
  holeCount: string
  courseName: string
  location: string
  category: TeeCategory
  teeName: string
  courseRating: string
  slopeRating: string
  holes: ManualHoleDraft[]
}

export type ManualValidation =
  | { ok: true; value: ManualCourseSelection }
  | {
    ok: false
    message: string
    fieldErrors: Partial<Record<ManualField, string>>
    holeErrors: Record<number, Partial<Record<HoleField, string>>>
    firstInvalid: string
  }

export type ManualField = 'holeCount' | 'courseName' | 'location' | 'teeName' | 'courseRating' | 'slopeRating'
export type HoleField = 'par' | 'strokeIndex' | 'distance'

const encoder = new TextEncoder()
const control = /\p{Cc}/u
const oneDecimal = /^\d{1,3}(?:[.,]\d)?$/
const integer = /^\d+$/

export function createManualDraft(holeCount: number): ManualCourseDraft {
  return {
    holeCount: String(holeCount), courseName: '', location: '', category: 'male', teeName: '', courseRating: '', slopeRating: '',
    holes: Array.from({ length: 36 }, (_, index) => ({ par: '', strokeIndex: String(index + 1), distance: '' })),
  }
}

export function resizeManualDraft(draft: ManualCourseDraft, holeCount: number): ManualCourseDraft {
  const holes = Array.from({ length: 36 }, (_, index) =>
    draft.holes[index] ?? { par: '', strokeIndex: String(index + 1), distance: '' })
  return { ...draft, holeCount: String(holeCount), holes }
}

export type CatalogSearch =
  | { ok: true; normalized: string }
  | { ok: false; normalized: string; message: string }

export function validateCatalogSearch(value: string): CatalogSearch {
  const normalized = value.trim().toLowerCase()
  if (new TextEncoder().encode(value).length > 80 || control.test(value)) {
    return { ok: false, normalized, message: 'Søket kan være opptil 80 byte og kan ikke inneholde kontrolltegn.' }
  }
  if (normalized && encoder.encode(normalized).length < 2) {
    return { ok: false, normalized, message: 'Skriv minst to byte, eller tøm søkefeltet.' }
  }
  return { ok: true, normalized }
}

function text(value: string, maximumBytes: number): string | null {
  const trimmed = value.trim()
  if (!trimmed || encoder.encode(trimmed).length > maximumBytes || control.test(trimmed)) return null
  return trimmed
}

function parseInteger(value: string, minimum: number, maximum: number): number | null {
  const trimmed = value.trim()
  if (!integer.test(trimmed)) return null
  const parsed = Number(trimmed)
  return Number.isSafeInteger(parsed) && parsed >= minimum && parsed <= maximum ? parsed : null
}

function parseRating(value: string): number | null {
  const trimmed = value.trim()
  if (!oneDecimal.test(trimmed)) return null
  const parsed = Number(trimmed.replace(',', '.'))
  return parsed >= 1 && parsed <= 100 ? parsed : null
}

export function validateManualDraft(draft: ManualCourseDraft): ManualValidation {
  const holeCount = parseInteger(draft.holeCount, 1, 36)
  const courseName = text(draft.courseName, 300)
  const location = draft.location.trim() ? text(draft.location, 500) : null
  const teeName = text(draft.teeName, 100)
  const rating = parseRating(draft.courseRating)
  const slope = parseInteger(draft.slopeRating, 55, 155)
  const fieldErrors: Partial<Record<ManualField, string>> = {}
  if (holeCount === null || draft.holes.length < holeCount) fieldErrors.holeCount = 'Antall hull må være et heltall fra 1 til 36.'
  if (!courseName) fieldErrors.courseName = 'Banenavn er påkrevd og kan være opptil 300 byte.'
  if (draft.location.trim() && !location) fieldErrors.location = 'Sted kan være opptil 500 byte og kan ikke inneholde kontrolltegn.'
  if (!teeName) fieldErrors.teeName = 'Navn på utslagssted er påkrevd og kan være opptil 100 byte.'
  if (rating === null) fieldErrors.courseRating = 'Baneverdi må være 1,0–100,0 med høyst én desimal.'
  if (slope === null) fieldErrors.slopeRating = 'Slope må være et heltall fra 55 til 155.'

  const holeErrors: Record<number, Partial<Record<HoleField, string>>> = {}
  const activeHoles = holeCount === null ? [] : draft.holes.slice(0, holeCount)
  const holes = activeHoles.map((hole, index) => {
    const par = parseInteger(hole.par, 2, 7)
    const strokeIndex = parseInteger(hole.strokeIndex, 1, activeHoles.length)
    const distance = hole.distance.trim() ? parseInteger(hole.distance, 1, 32_767) : null
    const errors: Partial<Record<HoleField, string>> = {}
    if (par === null) errors.par = 'Par må være et heltall fra 2 til 7.'
    if (strokeIndex === null) errors.strokeIndex = `Slagindeks må være et heltall fra 1 til ${activeHoles.length}.`
    if (hole.distance.trim() && distance === null) errors.distance = 'Avstand må være et positivt heltall i yards.'
    if (Object.keys(errors).length) holeErrors[index] = errors
    return { par, strokeIndex, distance }
  })
  const validIndexes = holes.map((hole) => hole.strokeIndex).filter((value): value is number => value !== null)
  if (validIndexes.length === activeHoles.length && new Set(validIndexes).size !== activeHoles.length) {
    for (const [index, hole] of holes.entries()) {
      if (hole.strokeIndex !== null && validIndexes.indexOf(hole.strokeIndex) !== validIndexes.lastIndexOf(hole.strokeIndex)) {
        holeErrors[index] = { ...holeErrors[index], strokeIndex: 'Slagindeksen er brukt flere ganger.' }
      }
    }
  }
  const firstField = (['holeCount', 'courseName', 'location', 'teeName', 'courseRating', 'slopeRating'] as const)
    .find((field) => fieldErrors[field])
  const firstHole = Object.entries(holeErrors)
    .flatMap(([index, errors]) => (['par', 'strokeIndex', 'distance'] as const)
      .filter((field) => errors[field]).map((field) => `holes.${index}.${field}`))[0]
  if (firstField || firstHole) {
    return {
      ok: false,
      message: 'Rett feltene som er markert. Alle hull må ha par og en komplett unik slagindeksrekke.',
      fieldErrors,
      holeErrors,
      firstInvalid: firstField ?? firstHole ?? 'courseName',
    }
  }
  if (holeCount === null || courseName === null || teeName === null || rating === null || slope === null) {
    throw new Error('Validerte banefelt mangler verdier')
  }
  return {
    ok: true,
    value: {
      source: 'manual', course_name: courseName, location,
      tee: {
        category: draft.category, name: teeName, course_rating: rating, slope_rating: slope,
        holes: holes.map((hole) => {
          if (hole.par === null || hole.strokeIndex === null) throw new Error('Validerte hull mangler verdier')
          return { par: hole.par, stroke_index: hole.strokeIndex, distance: hole.distance }
        }),
      },
    },
  }
}

export type ConfigurationFailure = 'stale' | 'not-draft' | 'tee-stale' | 'access' | 'retryable'

export function configurationFailure(error: Error | null): ConfigurationFailure | null {
  if (!error) return null
  if (!(error instanceof ApiHttpError)) return 'retryable'
  if (error.code === 'round_configuration_stale') return 'stale'
  if (error.code === 'round_not_draft') return 'not-draft'
  if (error.code === 'course_provider_tee_stale') return 'tee-stale'
  if (error.status === 401 || error.status === 403) return 'access'
  return 'retryable'
}

export function configurationErrorMessage(error: Error | null): string | null {
  switch (configurationFailure(error)) {
    case 'stale': return 'Runden ble endret et annet sted. Oppdaterte rundefakta er hentet; kontroller valgene og prøv igjen.'
    case 'not-draft': return 'Runden er ikke lenger et utkast og kan ikke endres.'
    case 'tee-stale': return 'Utslagsstedet finnes ikke lenger i leverandørdataene. Velg på nytt.'
    case 'access': return 'Du har ikke lenger tilgang til å endre denne runden.'
    case 'retryable': return 'Konfigurasjonen kunne ikke lagres. Innholdet er beholdt; prøv igjen.'
    default: return null
  }
}
