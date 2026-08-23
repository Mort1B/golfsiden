import { isCanonicalUuid } from '../../api/decoder'
import { ApiHttpError } from '../../api/http'
import type { MyTournament } from '../../api/tournaments'
import type { Tournament } from '../../api/types'

export const MANAGEMENT_SECTIONS = [
  { id: 'settings', label: 'Innstillinger' },
  { id: 'entrants', label: 'Deltakere' },
  { id: 'invitations', label: 'Invitasjoner' },
  { id: 'rounds', label: 'Runder' },
  { id: 'courses', label: 'Baner' },
  { id: 'pairings', label: 'Spillegrupper' },
  { id: 'lifecycle', label: 'Livsløp' },
] as const

export type ManagementSectionId = typeof MANAGEMENT_SECTIONS[number]['id']

export function managementSectionFromHash(hash: string): ManagementSectionId | null {
  const id = hash.startsWith('#') ? hash.slice(1) : ''
  return MANAGEMENT_SECTIONS.find((section) => section.id === id)?.id ?? null
}

interface ManagementAccessInput {
  tournamentId: string
  memberships: MyTournament[] | undefined
  membershipsPending: boolean
  membershipsError: Error | null
  tournament: Tournament | undefined
  tournamentPending: boolean
  tournamentError: Error | null
}

export type ManagementAccess =
  | { state: 'invalid' }
  | { state: 'loading' }
  | { state: 'missing' }
  | { state: 'forbidden' }
  | { state: 'error'; error: Error }
  | { state: 'ready'; tournament: Tournament }

function isHttpStatus(error: Error | null, status: number): boolean {
  return error instanceof ApiHttpError && error.status === status
}

export function resolveManagementAccess(input: ManagementAccessInput): ManagementAccess {
  if (!isCanonicalUuid(input.tournamentId)) return { state: 'invalid' }
  if (isHttpStatus(input.tournamentError, 404)) return { state: 'missing' }
  if (isHttpStatus(input.tournamentError, 403)) return { state: 'forbidden' }
  if (input.membershipsError) return { state: 'error', error: input.membershipsError }
  if (input.tournamentError) return { state: 'error', error: input.tournamentError }
  if (input.membershipsPending || input.tournamentPending) return { state: 'loading' }

  const membership = input.memberships?.find((entry) => entry.tournament.id === input.tournamentId)
  if (membership?.role !== 'admin') return { state: 'forbidden' }
  if (!input.tournament) return { state: 'error', error: new Error('Turneringsdata mangler. Prøv igjen.') }
  if (input.tournament.id !== input.tournamentId) {
    return { state: 'error', error: new Error('Turneringsdataene samsvarer ikke med adressen.') }
  }
  return { state: 'ready', tournament: input.tournament }
}
