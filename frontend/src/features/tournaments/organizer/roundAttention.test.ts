import { expect, it } from 'vitest'
import { completion, opening, round, tournament } from '../lifecycle/__tests__/fixtures'
import { completionAttention, openingAttention } from './roundAttention'

it('uses authoritative opening readiness, active tournament and exact destinations', () => {
  expect(openingAttention(tournament, round, opening)).toEqual([{ text: 'Runde 1 er klar til å åpnes.', href: `/manage/tournaments/${tournament.id}?round=${round.id}#lifecycle`, link: 'Se åpning' }])
  expect(openingAttention({ ...tournament, status: 'draft' }, round, opening).some(item => item.text.includes('klar'))).toBe(false)
  const items = openingAttention(tournament, round, { ...opening, ready: false,
    missing_flight_players: [{ player_id: 'one', display_name: 'One' }, { player_id: 'two', display_name: 'Two' }],
    missing_players: [{ player_id: 'three', display_name: 'Three' }],
    issues: [{ code: 'missing_flight_assignment', message: '' }, { code: 'missing_course', message: '' }, { code: 'missing_tee', message: '' }],
  })
  expect(items).toHaveLength(2)
  expect(items[0]?.text).toBe('Runde 1 har 2 spillere uten flight.')
  expect(items[0]?.href).toContain(`#pairings`)
  expect(items[1]?.href).toContain(`#courses`)
})

it('counts score-owning cards and separates incomplete cards from confirmation tasks', () => {
  const owner = completion().owners[0]
  if (!owner) throw new Error('Missing fixture owner')
  const items = completionAttention({ ...round, status: 'open' }, { ...completion(), ready_to_complete: false, owners: [
    { ...owner, complete: false, confirmed: false, holes_scored: 1 },
    { ...owner, owner: { type: 'team', id: 'team-one' }, complete: true, confirmed: false },
    { ...owner, owner: { type: 'team', id: 'team-two' }, complete: true, confirmed: true },
  ] })
  expect(items.map(item => item.text)).toEqual(['Runde 1 har 1 ufullstendig scorekort.', 'Runde 1: 1 scorekort må bekreftes.'])
  expect(items[1]?.href).toContain('owner_type=team')
  expect(items[1]?.href).toContain('owner=team-one')
  expect(items[1]?.href).toContain('view=summary')
})

it('links multiple confirmation cards to the exact round and never infers readiness from counts', () => {
  const validation = completion('completed')
  const owner = validation.owners[0]
  if (!owner) throw new Error('Missing fixture owner')
  const items = completionAttention({ ...round, status: 'completed' }, { ...validation, ready_to_lock: false,
    owners: [owner, { ...owner, owner: { type: 'player' as const, id: 'second' } }].map(item => ({ ...item, confirmed: false })),
  })
  expect(items).toHaveLength(1)
  expect(items[0]?.href).toBe(`/manage/tournaments/${tournament.id}?round=${round.id}#lifecycle`)
  const blocked = completionAttention({ ...round, status: 'completed' }, { ...validation, ready_to_lock: false })
  expect(blocked[0]?.text).toContain('trenger kontroll')
  expect(completionAttention({ ...round, status: 'completed' }, validation)[0]?.text).toContain('klar til å låses')
  expect(completionAttention({ ...round, status: 'open' }, completion())[0]?.text).toContain('klar til å fullføres')
  expect(completionAttention({ ...round, status: 'open' }, { ...completion(), ready_to_complete: false, owners: [] })[0]?.text).toContain('trenger kontroll')
})
