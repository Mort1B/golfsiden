import { decodeObject, decodeTimestamp, invalidData } from './decoder'

export type ScoreVisibilityMode = 'full' | 'front_nine'

export interface ScoreVisibility {
  mode: ScoreVisibilityMode
  observed_at: string
  hidden_until: string | null
}

export function decodeScoreVisibility(value: unknown, path: string, label: string): ScoreVisibility {
  const data = decodeObject(value, path, label)
  if (data.mode !== 'full' && data.mode !== 'front_nine') invalidData(label, `${path}.mode`)
  return {
    mode: data.mode,
    observed_at: decodeTimestamp(data.observed_at, `${path}.observed_at`, label),
    hidden_until: data.hidden_until === null
      ? null
      : decodeTimestamp(data.hidden_until, `${path}.hidden_until`, label),
  }
}

export function visibilityRefetchDelay(visibility: ScoreVisibility): number | null {
  if (visibility.mode !== 'front_nine' || visibility.hidden_until === null) return null
  const delay = Date.parse(visibility.hidden_until) - Date.parse(visibility.observed_at)
  return Number.isFinite(delay) && delay > 0 ? delay : null
}
