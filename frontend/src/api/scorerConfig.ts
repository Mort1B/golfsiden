import { isCanonicalUuid } from './decoder'

export type ScorerConfig =
  | { ready: true; userId: string }
  | { ready: false; message: string }

export function readScorerConfig(value: string | undefined): ScorerConfig {
  if (!value) return { ready: false, message: 'Scorer-ID mangler. Score kan vises, men ikke lagres.' }
  if (!isCanonicalUuid(value)) {
    return { ready: false, message: 'Scorer-ID er ugyldig. Score kan vises, men ikke lagres.' }
  }
  return { ready: true, userId: value }
}

export const scorerConfig = readScorerConfig(import.meta.env.VITE_SCORER_USER_ID)
