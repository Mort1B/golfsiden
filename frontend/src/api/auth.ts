import { decodeObject, decodeString, decodeTimestamp, decodeUuid, invalidData } from './decoder'

export type UserRole = 'admin' | 'scorer' | 'player' | 'viewer'

export interface AuthSession {
  user_id: string
  username: string
  display_name: string
  role: UserRole
  player_id: string | null
  expires_at: string
  csrf_token: string
}

function role(value: unknown): UserRole {
  if (value === 'admin' || value === 'scorer' || value === 'player' || value === 'viewer') return value
  return invalidData('sesjonsdata', 'session.role')
}

export function decodeAuthSession(value: unknown): AuthSession {
  const data = decodeObject(value, 'session', 'sesjonsdata')
  return {
    user_id: decodeUuid(data.user_id, 'session.user_id', 'sesjonsdata'),
    username: decodeString(data.username, 'session.username', 'sesjonsdata'),
    display_name: decodeString(data.display_name, 'session.display_name', 'sesjonsdata'),
    role: role(data.role),
    player_id: data.player_id === null
      ? null
      : decodeUuid(data.player_id, 'session.player_id', 'sesjonsdata'),
    expires_at: decodeTimestamp(data.expires_at, 'session.expires_at', 'sesjonsdata'),
    csrf_token: decodeString(data.csrf_token, 'session.csrf_token', 'sesjonsdata'),
  }
}

export const authKeys = {
  session: ['auth', 'session'] as const,
}
