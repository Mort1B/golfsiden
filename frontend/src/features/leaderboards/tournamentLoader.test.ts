import { QueryClient } from '@tanstack/react-query'
import { describe, expect, it } from 'vitest'
import type { Round, TournamentLeaderboard } from '../../api/types'
import { loadTournamentLeaderboardAfterRounds } from './tournamentLoader'

const tournamentId = '00000000-0000-0000-0000-000000002001'
const roundId = '00000000-0000-0000-0000-000000004001'

function round(status: Round['status']): Round {
  return {
    id: roundId,
    tournament_id: tournamentId,
    round_number: 1,
    name: 'Finale',
    round_date: '2026-09-01',
    course_id: null,
    course_name: 'Testbane',
    tee_id: null,
    tee_name: 'Gul',
    number_of_holes: 18,
    status,
    handicap_enabled: true,
    handicap_allowance_percent: 100,
    scoring_format: 'individual_stroke_play',
    created_at: '2026-09-01T10:00:00Z',
    updated_at: '2026-09-01T10:00:00Z',
  }
}

function leaderboard(currentRoundId: string | null, includedRoundIds: string[]): TournamentLeaderboard {
  return { tie_break_policy: 'shared_positions', final_round_number: 2,
    tournament_id: tournamentId,
    metric: 'gross',
    required_counted_rounds: 1,
    mandatory_round_id: null,
    current_round_id: currentRoundId,
    included_round_ids: includedRoundIds,
    visibility: { mode: 'full' },
    entries: [],
  }
}

describe('tournament leaderboard lifecycle coordination', () => {
  it.each([
    {
      name: 'open to completed',
      cached: round('open'),
      refreshed: round('completed'),
      response: leaderboard(null, [roundId]),
    },
    {
      name: 'draft to open',
      cached: round('draft'),
      refreshed: round('open'),
      response: leaderboard(roundId, []),
    },
  ])('refreshes rounds before validating a $name transition', async ({ cached, refreshed, response }) => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const roundsQueryKey = ['private-workspace', 'user', 'tournaments', tournamentId, 'rounds'] as const
    const events: string[] = []
    queryClient.setQueryData(roundsQueryKey, [cached])

    const result = await loadTournamentLeaderboardAfterRounds({
      queryClient,
      roundsQueryKey,
      loadRounds: async () => {
        events.push('rounds')
        return [refreshed]
      },
      loadLeaderboard: async () => {
        events.push('leaderboard')
        return response
      },
    })

    expect(result).toBe(response)
    expect(events).toEqual(['rounds', 'leaderboard'])
    expect(queryClient.getQueryData(roundsQueryKey)).toEqual([refreshed])
  })
})

it('the nested rounds request clears private projections even when its parent was cancelled', async () => {
  const { ApiHttpError } = await import('../../api/http')
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const key = ['private-workspace', 'user', 'tournaments', tournamentId, 'rounds'] as const
  const cardKey = ['private-workspace', 'user', 'rounds', roundId, 'scorecards', 'read', 'player', 'player'] as const
  client.setQueryData(key, [round('open')]); client.setQueryData(cardKey, { round_id: roundId, private: 'card' })
  const controller = new AbortController()
  let fail: (error: unknown) => void = () => undefined
  const pending = new Promise<Round[]>((_resolve, reject) => { fail = reject })
  const request = loadTournamentLeaderboardAfterRounds({ queryClient: client, roundsQueryKey: key, signal: controller.signal,
    loadRounds: () => pending, loadLeaderboard: async () => leaderboard(roundId, []) })
  controller.abort(); fail(new ApiHttpError(403, 'forbidden', 'Access denied'))
  await expect(request).rejects.toThrow()
  expect(client.getQueryState(key)?.error).toMatchObject({ status: 403 })
  expect(client.getQueryData(cardKey)).toBeUndefined()
})
