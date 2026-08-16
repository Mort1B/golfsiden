import { describe, expect, it } from 'vitest'
import { addRound, createInitialDraft, removeRound, toOnboardingRequest, updateRound } from './wizardState'
import { hasErrors, validateAll } from './validation'

describe('onboarding wizard state', () => {
  it('keeps API round numbers contiguous after removal', () => {
    let draft = addRound(addRound(createInitialDraft('2026-09-01')))
    const removedKey = draft.rounds[1]?.key ?? ''
    draft = removeRound(draft, removedKey)
    const lastKey = draft.rounds[1]?.key ?? ''
    draft = updateRound(draft, lastKey, { name: 'Finale' })

    expect(toOnboardingRequest(draft).rounds).toEqual([
      expect.objectContaining({ round_number: 1, name: 'Runde 1' }),
      expect.objectContaining({ round_number: 2, name: 'Finale' }),
    ])
  })

  it('serializes the exact nested contract without trimming the password', () => {
    const draft = createInitialDraft('2026-09-01')
    draft.tournament = { name: ' Tur ', description: ' Beskrivelse ', startDate: '2026-09-01', endDate: '2026-09-03' }
    draft.creator = { displayName: ' Morten ', email: ' MORTEN@EXAMPLE.NO ', password: ' passord med rom ', handicap: '12.4' }

    expect(toOnboardingRequest(draft)).toMatchObject({
      creator: {
        account: { email: 'morten@example.no', password: ' passord med rom ' },
        player: { display_name: 'Morten', handicap_index: 12.4 },
      },
      tournament: { name: 'Tur', description: 'Beskrivelse' },
    })
  })

  it('mirrors date, byte, password, handicap, and round bounds', () => {
    const draft = createInitialDraft('2026-09-01')
    draft.tournament.name = 'x'.repeat(121)
    draft.tournament.endDate = '2026-08-31'
    draft.creator = { displayName: '', email: 'ugyldig', password: 'kort', handicap: '55' }
    const firstRound = draft.rounds[0]
    if (!firstRound) throw new Error('initial draft must have one round')
    draft.rounds[0] = { ...firstRound, date: '2027-01-01' }

    expect(hasErrors(validateAll(draft, '2026-09-01'))).toBe(true)
    expect(validateAll(draft, '2026-09-01')).toMatchObject({
      'tournament.name': expect.any(String),
      'tournament.endDate': expect.any(String),
      'creator.email': expect.any(String),
      'creator.password': expect.any(String),
      'creator.handicap': expect.any(String),
    })
  })
})
