import { QueryClient } from '@tanstack/react-query'
import { describe, expect, it } from 'vitest'
import { clearPrivateWorkspace, privateWorkspaceKeys } from './privateWorkspace'
import { tournamentKeys } from './tournaments'
import { pairingKeys } from './pairings'
import { scoringKeys } from './scorecards'

describe('private workspace cache', () => {
  it('uses one user-owned hierarchy for readiness and tournament reads', () => {
    expect(privateWorkspaceKeys.completion('user-one', 'round-one')).toEqual([
      'private-workspace', 'user-one', 'rounds', 'round-one', 'completion-validation',
    ])
    expect(privateWorkspaceKeys.scoreAccess('user-one', 'round-one')).toEqual([
      'private-workspace', 'user-one', 'rounds', 'round-one', 'score-access',
    ])
    expect(pairingKeys.detail('user-one', 'round-one')).toEqual([
      'private-workspace', 'user-one', 'rounds', 'round-one', 'pairings',
    ])
  })

  it('removes every user workspace including scorecards without touching public caches', () => {
    const queryClient = new QueryClient()
    queryClient.setQueryData(tournamentKeys.detail('user-one', 'tour-one'), { name: 'Første' })
    queryClient.setQueryData(tournamentKeys.detail('user-two', 'tour-two'), { name: 'Andre' })
    queryClient.setQueryData(privateWorkspaceKeys.invitations('user-one', 'tour-one'), [{ id: 'invite-admin' }])
    queryClient.setQueryData(['invitations', 'preview', 'invite-one'], { name: 'Forhåndsvisning' })
    const scorecardKey = scoringKeys.scorecard('user-one', 'round-one', { type: 'player', id: 'player-one' })
    queryClient.setQueryData(scorecardKey, { gross_total: 72 })

    clearPrivateWorkspace(queryClient)

    expect(queryClient.getQueriesData({ queryKey: privateWorkspaceKeys.root })).toEqual([])
    expect(queryClient.getQueryData(['invitations', 'preview', 'invite-one'])).toEqual({ name: 'Forhåndsvisning' })
    expect(queryClient.getQueryData(scorecardKey)).toBeUndefined()
  })
})
