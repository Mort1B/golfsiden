import { describe, expect, it } from 'vitest'
import { addRound, createInitialDraft, removeRound, toOnboardingRequest, updateCountedRounds, updateRound } from './wizardState'
import { hasErrors, validateAll } from './validation'

describe('onboarding wizard state', () => {
  it('keeps API round numbers contiguous after removal', () => {
    let draft = addRound(addRound(createInitialDraft('2026-09-01')))
    draft.creator.handicap = '0,0'
    const removedKey = draft.rounds[1]?.key ?? ''
    draft = removeRound(draft, removedKey)
    const lastKey = draft.rounds[1]?.key ?? ''
    draft = updateRound(draft, lastKey, { name: 'Finale' })

    expect(toOnboardingRequest(draft).rounds).toEqual([
      expect.objectContaining({ round_number: 1, name: 'Runde 1' }),
      expect.objectContaining({ round_number: 2, name: 'Finale' }),
    ])
  })

  it('tracks all rounds until a smaller explicit count is chosen, then clamps on removal', () => {
    let draft = createInitialDraft('2026-09-01')
    draft = addRound(draft)
    expect(draft.countedRounds).toBe(2)

    draft = updateCountedRounds(draft, 1)
    draft = addRound(draft)
    expect(draft.countedRounds).toBe(1)

    draft = updateCountedRounds(draft, 2)
    const removedKey = draft.rounds[2]?.key ?? ''
    draft = removeRound(draft, removedKey)
    expect(draft.countedRounds).toBe(2)

    draft = addRound(draft)
    expect(draft.countedRounds).toBe(2)
    expect(draft.countedRoundsMode).toBe('custom')
  })

  it('clamps an all-round selection when a round is removed', () => {
    let draft = addRound(addRound(createInitialDraft('2026-09-01')))
    const removedKey = draft.rounds[2]?.key ?? ''
    draft = removeRound(draft, removedKey)
    expect(draft.countedRounds).toBe(2)
    expect(draft.countedRoundsMode).toBe('all')
  })

  it('serializes the exact nested contract without trimming the password', () => {
    const draft = createInitialDraft('2026-09-01')
    draft.tournament = { name: ' Tur ', description: ' Beskrivelse ', startDate: '2026-09-01', endDate: '2026-09-03' }
    draft.creator = { displayName: ' Morten ', username: ' MORTEN_14 ', password: ' passord med rom ', handicap: '12,4' }

    expect(toOnboardingRequest(draft)).toMatchObject({
      creator: {
        account: { username: 'morten_14', password: ' passord med rom ' },
        player: { display_name: 'Morten', handicap_index: 12.4 },
      },
      tournament: { name: 'Tur', description: 'Beskrivelse', counted_rounds: 1 },
    })
  })

  it('preserves foursomes in the exact onboarding request and validation', () => {
    const draft = createInitialDraft('2026-09-01')
    const firstRound = draft.rounds[0]
    if (!firstRound) throw new Error('initial draft must have one round')
    draft.rounds[0] = { ...firstRound, scoringFormat: 'two_player_foursomes' }
    expect(toOnboardingRequest({
      ...draft,
      creator: { ...draft.creator, handicap: '12,4' },
    }).rounds[0]?.scoring_format).toBe('two_player_foursomes')
    expect(validateAll(draft, '2026-09-01')).not.toHaveProperty('rounds.round-1.scoringFormat')
  })

  it('mirrors date, byte, password, handicap, and round bounds', () => {
    const draft = createInitialDraft('2026-09-01')
    draft.tournament.name = 'x'.repeat(121)
    draft.countedRounds = 0
    draft.tournament.endDate = '2026-08-31'
    draft.creator = { displayName: '', username: 'æøå', password: 'kort', handicap: '55' }
    const firstRound = draft.rounds[0]
    if (!firstRound) throw new Error('initial draft must have one round')
    draft.rounds[0] = { ...firstRound, date: '2027-01-01' }

    expect(hasErrors(validateAll(draft, '2026-09-01'))).toBe(true)
    expect(validateAll(draft, '2026-09-01')).toMatchObject({
      'tournament.name': expect.any(String),
      'tournament.endDate': expect.any(String),
      'rounds.countedRounds': expect.any(String),
      'creator.username': expect.any(String),
      'creator.password': expect.any(String),
      'creator.handicap': expect.any(String),
    })
  })
})
