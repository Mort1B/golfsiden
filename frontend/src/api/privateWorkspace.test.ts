import { QueryClient } from '@tanstack/react-query'
import { describe, expect, it } from 'vitest'
import { clearPrivateWorkspace, privateWorkspaceKeys } from './privateWorkspace'
import { tournamentKeys } from './tournaments'

describe('private workspace cache', () => {
  it('uses one user-owned hierarchy for readiness and tournament reads', () => {
    expect(privateWorkspaceKeys.completion('user-one', 'round-one')).toEqual([
      'private-workspace', 'user-one', 'rounds', 'round-one', 'completion-validation',
    ])
    expect(privateWorkspaceKeys.scoreAccess('user-one', 'round-one')).toEqual([
      'private-workspace', 'user-one', 'rounds', 'round-one', 'score-access',
    ])
  })

  it('removes every user workspace without touching public or scorecard caches', () => {
    const queryClient = new QueryClient()
    queryClient.setQueryData(tournamentKeys.detail('user-one', 'tour-one'), { name: 'Første' })
    queryClient.setQueryData(tournamentKeys.detail('user-two', 'tour-two'), { name: 'Andre' })
    queryClient.setQueryData(privateWorkspaceKeys.invitations('user-one', 'tour-one'), [{ id: 'invite-admin' }])
    queryClient.setQueryData(['invitations', 'preview', 'invite-one'], { name: 'Forhåndsvisning' })
    queryClient.setQueryData(['scorecards', 'round-one', 'player', 'player-one'], { gross_total: 72 })

    clearPrivateWorkspace(queryClient)

    expect(queryClient.getQueriesData({ queryKey: privateWorkspaceKeys.root })).toEqual([])
    expect(queryClient.getQueryData(['invitations', 'preview', 'invite-one'])).toEqual({ name: 'Forhåndsvisning' })
    expect(queryClient.getQueryData(['scorecards', 'round-one', 'player', 'player-one'])).toEqual({ gross_total: 72 })
  })
})
