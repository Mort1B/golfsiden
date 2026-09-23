import { expect, type Page } from '@playwright/test'
import { offlineFixture } from './offlineSupport'
import { decodeObject, decodeUuid } from '../src/api/decoder'
import type { Round } from '../src/api/types'
import { decodeRound, decodeTournamentRounds, decodeTournament } from '../src/api/tournaments/decoders'

export async function contextFixture(page: Page) {
  const first = await offlineFixture(page)
  const day = new Date(Date.now() + 86400000).toISOString().slice(0, 10)
  const result = await first.mutate('/api/tournaments', {
    request_id: crypto.randomUUID(),
    tournament: { name: `Andre turnering med langt navn ${Date.now()}`, description: '', start_date: day, end_date: day, counted_rounds: 1, mandatory_round_number: null },
    rounds: [1, 2].map(number => ({ round_number: number, name: `Andre turnering runde ${number}`, round_date: day, scoring_format: 'individual_stroke_play' })),
  })
  const id = decodeUuid(decodeObject(await result.json(), 'receipt').tournament_id, 'id')
  const rounds = decodeTournamentRounds(await (await page.request.get(`/api/tournaments/${id}/rounds`)).json(), id)
  if (!first.auth.player_id) throw new Error('Missing second player')
  async function configure(draft: Round) {
    const round = decodeRound(await (await first.mutate(`/api/rounds/${draft.id}/course-configuration`, {
      expected_round_updated_at: draft.updated_at, selection: { source: 'manual', course_name: 'Andre bane', location: null,
        tee: { category: 'male', name: 'Gul', course_rating: 72, slope_rating: 113, holes: Array.from({ length: 18 }, (_, i) => ({ par: 4, stroke_index: i + 1, distance: null })) } },
    }, 'PUT')).json())
    await first.mutate(`/api/rounds/${round.id}/pairings`, { expected_round_updated_at: round.updated_at, teams: [], flights: [{ id: crypto.randomUUID(), name: 'Flight', starting_hole: 1, tee_time: null, members: [{ player_id: first.auth.player_id }] }], legacy_conversions: [] }, 'PUT')
    return round
  }
  const configured: Round[] = []
  for (const draft of rounds) configured.push(await configure(draft))
  const round = configured[0]
  if (!round) throw new Error('Missing second round')
  const trip = decodeTournament(await (await page.request.get(`/api/tournaments/${id}`)).json())
  await first.mutate(`/api/tournaments/${id}/start`, { expected_tournament_updated_at: trip.updated_at })
  for (const item of configured) await first.mutate(`/api/rounds/${item.id}/open`)
  const list: unknown = await (await page.request.get('/api/tournaments')).json()
  expect(Array.isArray(list)).toBe(true)
  return { first, second: { id, round, rounds: configured }, list }
}
