import { decodeBoolean, decodeObject, decodeUuid, invalidData } from './decoder'
import { jsonRequest, requestDecoded } from './http'
import type { OnboardingRequest } from './onboarding'
export type TournamentPlanRequest = Pick<OnboardingRequest, 'tournament' | 'rounds'>
export interface CreateTournamentRequest extends TournamentPlanRequest { request_id: string }
export interface TournamentCreationReceipt { request_id: string; tournament_id: string; created: boolean }
export function decodeCreationReceipt(value: unknown, requestId: string): TournamentCreationReceipt {
  const r = decodeObject(value, 'creation')
  const id = decodeUuid(r.request_id, 'creation.request_id')
  if (id !== requestId) return invalidData('opprettingsdata', 'creation.request_id')
  return { request_id: id, tournament_id: decodeUuid(r.tournament_id, 'creation.tournament_id'), created: decodeBoolean(r.created, 'creation.created') }
}
export const creationApi = { create: (input: CreateTournamentRequest, csrf: string) =>
  requestDecoded('/api/tournaments', (value) => decodeCreationReceipt(value, input.request_id), jsonRequest('POST', input, csrf)) }
