import { invalidData } from '../decoder'

export function decodeScoreRevision(value: unknown): string {
  if (typeof value !== 'string' || !/^[1-9][0-9]{0,18}$/.test(value)
    || (value.length === 19 && value > '9223372036854775807')) {
    return invalidData('scorekortdata', 'score.revision')
  }
  return value
}
