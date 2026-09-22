import { decodeCompletion, decodeListing, decodePlayerListing, decodeTable } from './matchPlay/decoders'
export { decodeCompletion, decodeListing, decodePlayerListing, decodeTable } from './matchPlay/decoders'
import { decodeObject, invalidData } from './decoder'
import { jsonRequest, requestDecoded } from './http'
import { privateWorkspaceKeys } from './privateWorkspace'
import { decodeExpectedRound } from './tournaments/decoders'
import { decodeScoreRevision } from './scorecards/revision'
import { decodeCard } from './matchPlay/cardDecoder'
import { exact } from './matchPlay/eventDecoder'
import type { MatchAcknowledgement, MatchPair, MatchRequest } from './matchPlay/contracts'
export type * from './matchPlay/contracts'
export { decodeCard } from './matchPlay/cardDecoder'
export const matchKeys = {
  list: (user: string, round: string) => [...privateWorkspaceKeys.user(user), 'rounds', round, 'match-play', 'read-list'] as const,
  listForPlayer: (user: string, round: string, player: string) => [...matchKeys.list(user, round), 'player', player] as const,
  read: (user: string, round: string, match: string) => [...privateWorkspaceKeys.user(user), 'rounds', round, 'match-play', 'read', match] as const,
  scoring: (user: string, round: string, match: string) => [...privateWorkspaceKeys.user(user), 'rounds', round, 'match-play', 'scoring', match] as const,
  completion: (user: string, round: string) => [...privateWorkspaceKeys.user(user), 'rounds', round, 'match-play', 'completion'] as const,
  table: (user: string, tournament: string) => [...privateWorkspaceKeys.user(user), 'tournaments', tournament, 'match-table'] as const,
}
const base = (round: string) => `/api/rounds/${round}/match-play`
export const matchApi = {
  list: (round: string, signal?: AbortSignal) => requestDecoded(`${base(round)}/matches`, v => decodeListing(v, round), { signal }),
  listForPlayer: (round: string, player: string, signal?: AbortSignal) => requestDecoded(`${base(round)}/matches?${new URLSearchParams({ player_id: player })}`, v => decodePlayerListing(v, round, player), { signal }),
  read: (round: string, match: string, signal?: AbortSignal) => requestDecoded(`${base(round)}/matches/${match}`, v => decodeCard(v, round, match), { signal }),
  scoring: (round: string, match: string, signal?: AbortSignal) => requestDecoded(`${base(round)}/matches/${match}/scoring`, v => decodeCard(v, round, match, true), { signal }),
  completion: (round: string) => requestDecoded(`${base(round)}/completion`, v => decodeCompletion(v, round)),
  table: (tournament: string, signal?: AbortSignal) => requestDecoded(`/api/tournaments/${tournament}/match-table`, v => decodeTable(v, tournament), { signal }),
  settings: (round: string, expected: string, enabled: boolean, csrf: string) => requestDecoded(`${base(round)}/settings`, v => decodeExpectedRound(v, round), jsonRequest('PUT', { expected_round_updated_at: expected, handicap_enabled: enabled }, csrf)),
  assign: (round: string, expected: string, matches: MatchPair[], csrf: string) => requestDecoded(`${base(round)}/matches`, v => decodeListing(v, round), jsonRequest('PUT', { expected_round_updated_at: expected, matches }, csrf)),
  command: (round: string, match: string, request: MatchRequest, csrf: string, signal?: AbortSignal): Promise<MatchAcknowledgement> => requestDecoded(`${base(round)}/matches/${match}/commands`, v => {
    const d = decodeObject(v, 'ack'); exact(d, ['request_id', 'match_id', 'applied_revision'])
    const revision = decodeScoreRevision(d.applied_revision)
    if (d.request_id !== request.request_id || d.match_id !== match || BigInt(revision) !== BigInt(request.expected_revision) + 1n) invalidData('matchdata', 'ack.identity')
    return { request_id: request.request_id, match_id: match, applied_revision: revision }
  }, { ...jsonRequest('POST', request, csrf), signal }),
}
