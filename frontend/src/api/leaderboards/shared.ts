import { decodeInteger, decodeObject, decodeString, decodeUuid, invalidData } from '../decoder'
import { isScoringFormat } from '../scoringFormats'
import type { LeaderboardMember, LeaderboardMetric, LeaderboardOwner, ParticipantStatus, RoundStatus, ScoringFormat } from '../types'

export function invalidLeaderboard(path: string): never { return invalidData('resultatdata', path) }
export function nullablePosition(value: unknown, path: string): number | null {
  return value === null ? null : decodeInteger(value, path, 1, undefined, 'resultatdata')
}
export function decodeMetric(value: unknown, path: string): LeaderboardMetric {
  if (value === 'gross' || value === 'net') return value
  return invalidLeaderboard(path)
}
export function decodeRoundStatus(value: unknown, path: string): RoundStatus {
  if (value === 'draft' || value === 'open' || value === 'completed' || value === 'locked') return value
  return invalidLeaderboard(path)
}
export function decodeScoringFormat(value: unknown, path: string): ScoringFormat {
  if (isScoringFormat(value)) return value
  return invalidLeaderboard(path)
}
export function decodeParticipantStatus(value: unknown, path: string): ParticipantStatus {
  if (value === 'active' || value === 'withdrawn') return value
  return invalidLeaderboard(path)
}
export function decodeLeaderboardOwner(value: unknown, path: string): LeaderboardOwner {
  const data = decodeObject(value, path, 'resultatdata')
  if (data.type === 'player') return { type: 'player', id: decodeUuid(data.id, `${path}.id`, 'resultatdata') }
  if (data.type === 'team') return { type: 'team', id: decodeUuid(data.id, `${path}.id`, 'resultatdata') }
  return invalidLeaderboard(`${path}.type`)
}
export function decodeLeaderboardMember(value: unknown, path: string): LeaderboardMember {
  const data = decodeObject(value, path, 'resultatdata')
  return {
    player_id: decodeUuid(data.player_id, `${path}.player_id`, 'resultatdata'),
    display_name: decodeString(data.display_name, `${path}.display_name`, 'resultatdata'),
    display_order: data.display_order === null ? null : decodeInteger(data.display_order, `${path}.display_order`, undefined, undefined, 'resultatdata'),
  }
}
