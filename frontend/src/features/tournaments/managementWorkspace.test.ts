import { describe, expect, it } from 'vitest'
import { ApiHttpError } from '../../api/http'
import type { MyTournament } from '../../api/tournaments'
import type { Tournament } from '../../api/types'
import { MANAGEMENT_SECTIONS, managementSectionFromHash, resolveManagementAccess } from './managementWorkspace'

const tournamentId = '00000000-0000-0000-0000-000000000001'
const tournament: Tournament = {
  id: tournamentId,
  name: 'Langhelg',
  description: '',
  start_date: '2026-09-01',
  end_date: '2026-09-04',
  number_of_rounds: 3,
  status: 'draft',
  scoring_mode: 'combined',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
}
const admin: MyTournament = { tournament, role: 'admin', player_id: null }

function access(overrides: Partial<Parameters<typeof resolveManagementAccess>[0]> = {}) {
  return resolveManagementAccess({
    tournamentId,
    memberships: [admin],
    membershipsPending: false,
    membershipsError: null,
    tournament,
    tournamentPending: false,
    tournamentError: null,
    ...overrides,
  })
}

describe('management workspace sections', () => {
  it('keeps the exact semantic anchor contract', () => {
    expect(MANAGEMENT_SECTIONS).toEqual([
      { id: 'settings', label: 'Innstillinger' },
      { id: 'entrants', label: 'Deltakere' },
      { id: 'invitations', label: 'Invitasjoner' },
      { id: 'rounds', label: 'Runder' },
      { id: 'courses', label: 'Baner' },
      { id: 'pairings', label: 'Spillegrupper' },
      { id: 'lifecycle', label: 'Livsløp' },
    ])
  })

  it('recognizes only exact management section hashes', () => {
    for (const section of MANAGEMENT_SECTIONS) {
      expect(managementSectionFromHash(`#${section.id}`)).toBe(section.id)
    }
    expect(managementSectionFromHash('')).toBeNull()
    expect(managementSectionFromHash('#unknown')).toBeNull()
    expect(managementSectionFromHash('#Invitations')).toBeNull()
    expect(managementSectionFromHash('invitations')).toBeNull()
  })
})

describe('management workspace access', () => {
  it('rejects noncanonical IDs before considering query state', () => {
    expect(access({ tournamentId: 'not-a-uuid', membershipsPending: true })).toEqual({ state: 'invalid' })
  })

  it('distinguishes missing tournaments from forbidden access', () => {
    expect(access({ tournamentError: new ApiHttpError(404, 'not_found', 'missing') })).toEqual({ state: 'missing' })
    expect(access({ tournamentError: new ApiHttpError(403, 'forbidden', 'denied') })).toEqual({ state: 'forbidden' })
    expect(access({ memberships: [{ ...admin, role: 'viewer' }] })).toEqual({ state: 'forbidden' })
  })

  it('waits for both authority sources and exposes retryable errors', () => {
    expect(access({ tournamentPending: true, tournament: undefined })).toEqual({ state: 'loading' })
    const error = new Error('offline')
    expect(access({ membershipsError: error })).toEqual({ state: 'error', error })
  })

  it('requires matching detail data before enabling the workspace', () => {
    expect(access()).toEqual({ state: 'ready', tournament })
    expect(access({ tournament: undefined })).toMatchObject({ state: 'error' })
    expect(access({ tournament: { ...tournament, id: '00000000-0000-0000-0000-000000000002' } }))
      .toMatchObject({ state: 'error' })
  })
})
