import { decodeObject, invalidData } from './decoder'

export type ScoreVisibilityMode = 'full' | 'front_nine'

export interface ScoreVisibility {
  mode: ScoreVisibilityMode
}

export function decodeScoreVisibility(value: unknown, path: string, label: string): ScoreVisibility {
  const data = decodeObject(value, path, label)
  if (data.mode !== 'full' && data.mode !== 'front_nine') invalidData(label, `${path}.mode`)
  return { mode: data.mode }
}
