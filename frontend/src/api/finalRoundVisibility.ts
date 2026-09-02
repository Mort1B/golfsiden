import { decodeBoolean, decodeObject, decodeTimestamp, decodeUuid, invalidData } from './decoder'
import { requestDecoded } from './http'
import { privateWorkspaceKeys } from './privateWorkspace'

export interface FinalRoundVisibility {
  tournament_id: string
  back_nine_hidden: boolean
  visibility_updated_at: string
}

export const finalRoundVisibilityKeys = {
  detail: (userId: string, tournamentId: string) => [
    ...privateWorkspaceKeys.user(userId),
    'tournaments',
    tournamentId,
    'final-round-visibility',
  ] as const,
}

export function decodeFinalRoundVisibility(
  value: unknown,
  expectedTournamentId: string,
): FinalRoundVisibility {
  const data = decodeObject(value, 'visibility', 'synlighetsdata')
  const decoded: FinalRoundVisibility = {
    tournament_id: decodeUuid(data.tournament_id, 'visibility.tournament_id', 'synlighetsdata'),
    back_nine_hidden: decodeBoolean(data.back_nine_hidden, 'visibility.back_nine_hidden', 'synlighetsdata'),
    visibility_updated_at: decodeTimestamp(
      data.visibility_updated_at,
      'visibility.visibility_updated_at',
      'synlighetsdata',
    ),
  }
  if (decoded.tournament_id !== expectedTournamentId) {
    invalidData('synlighetsdata', 'visibility.tournament_id identity')
  }
  return decoded
}

export const finalRoundVisibilityApi = {
  get: (tournamentId: string) => requestDecoded(
    `/api/tournaments/${tournamentId}/final-round-visibility`,
    (value) => decodeFinalRoundVisibility(value, tournamentId),
  ),
  update: (
    tournamentId: string,
    input: { back_nine_hidden: boolean; expected_visibility_updated_at: string },
    csrfToken: string,
  ) => requestDecoded(
    `/api/tournaments/${tournamentId}/final-round-visibility`,
    (value) => decodeFinalRoundVisibility(value, tournamentId),
    {
      method: 'PATCH',
      headers: { 'content-type': 'application/json', 'x-csrf-token': csrfToken },
      body: JSON.stringify(input),
    },
  ),
}
