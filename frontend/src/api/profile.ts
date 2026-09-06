import { decodeBoolean, decodeInteger, decodeNumber, decodeObject, decodeString, decodeTimestamp, decodeUuid, invalidData } from './decoder'
import { privateWorkspaceKeys } from './privateWorkspace'

export interface Profile {
  user_id: string
  username: string
  display_name: string
  version: number
  player_id: string | null
  handicap: number | null
  player_active: boolean | null
  player_updated_at: string | null
}
export interface ProfileDetails {
  version: number
  player_updated_at: string | null
  display_name: string
  handicap: number | null
}
export function decodeProfile(value: unknown, userId: string): Profile {
  const data = decodeObject(value, 'profile')
  const result = {
    user_id: decodeUuid(data.user_id, 'profile.user_id'),
    username: decodeString(data.username, 'profile.username'),
    display_name: decodeString(data.display_name, 'profile.display_name'),
    version: decodeInteger(data.version, 'profile.version', 0),
    player_id: data.player_id === null ? null : decodeUuid(data.player_id, 'profile.player_id'),
    handicap: data.handicap === null ? null : decodeNumber(data.handicap, 'profile.handicap', -10, 54),
    player_active: data.player_active === null ? null : decodeBoolean(data.player_active, 'profile.player_active'),
    player_updated_at: data.player_updated_at === null ? null : decodeTimestamp(data.player_updated_at, 'profile.player_updated_at'),
  }
  if (result.user_id !== userId || (result.player_id === null
    ? result.handicap !== null || result.player_active !== null || result.player_updated_at !== null
    : result.handicap === null || result.player_active === null || result.player_updated_at === null)) invalidData('profildata', 'profile.identity')
  if (result.handicap !== null && Math.abs(result.handicap * 10 - Math.round(result.handicap * 10)) > 1e-8) invalidData('profildata', 'profile.handicap')
  return result
}
export const profileKeys = { me: (userId: string) => [...privateWorkspaceKeys.user(userId), 'profile'] as const }
