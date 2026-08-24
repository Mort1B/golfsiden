import type { CreatorDraft, RoundDraft, TournamentDraft, WizardDraft } from './wizardState'
import { parseHandicap } from '../handicap/format'
import { isScoringFormat } from '../../api/scoringFormats'

export type FieldErrors = Record<string, string>

const encoder = new TextEncoder()

function byteLength(value: string): number {
  return encoder.encode(value).length
}

function validateName(value: string, label: string): string | null {
  if (!value.trim()) return `${label} må fylles ut.`
  if (value.includes('\0')) return `${label} inneholder ugyldige tegn.`
  if (byteLength(value) > 120) return `${label} kan være maks 120 byte.`
  return null
}

export function validateTournament(tournament: TournamentDraft, today: string): FieldErrors {
  const errors: FieldErrors = {}
  const nameError = validateName(tournament.name, 'Turneringsnavn')
  if (nameError) errors['tournament.name'] = nameError
  if (tournament.description.includes('\0') || byteLength(tournament.description) > 2_000) {
    errors['tournament.description'] = 'Beskrivelsen kan være maks 2000 byte og kan ikke inneholde ugyldige tegn.'
  }
  if (!tournament.startDate) errors['tournament.startDate'] = 'Velg startdato.'
  if (!tournament.endDate) errors['tournament.endDate'] = 'Velg sluttdato.'
  if (tournament.startDate && tournament.endDate && tournament.endDate < tournament.startDate) {
    errors['tournament.endDate'] = 'Sluttdato kan ikke være før startdato.'
  } else if (tournament.endDate && tournament.endDate < today) {
    errors['tournament.endDate'] = 'Sluttdato kan ikke være i fortiden.'
  }
  return errors
}

export function validateRounds(rounds: RoundDraft[], tournament: TournamentDraft): FieldErrors {
  const errors: FieldErrors = {}
  if (rounds.length < 1 || rounds.length > 30) errors.rounds = 'Turneringen må ha mellom 1 og 30 runder.'
  for (const round of rounds) {
    const prefix = `rounds.${round.key}`
    const nameError = validateName(round.name, 'Rundenavn')
    if (nameError) errors[`${prefix}.name`] = nameError
    if (!round.date) {
      errors[`${prefix}.date`] = 'Velg rundedato.'
    } else if (round.date < tournament.startDate || round.date > tournament.endDate) {
      errors[`${prefix}.date`] = 'Rundedato må være innenfor turneringsperioden.'
    }
    if (!isScoringFormat(round.scoringFormat)) {
      errors[`${prefix}.scoringFormat`] = 'Velg en støttet spilleform.'
    }
  }
  return errors
}

export function validateCreator(creator: CreatorDraft): FieldErrors {
  const errors: FieldErrors = {}
  const nameError = validateName(creator.displayName, 'Visningsnavn')
  if (nameError) errors['creator.displayName'] = nameError
  if (!/^[A-Za-z0-9_-]{3,32}$/.test(creator.username.trim())) {
    errors['creator.username'] = 'Bruk 3–32 bokstaver, tall, bindestrek eller understrek.'
  }
  const passwordBytes = byteLength(creator.password)
  if (passwordBytes < 12 || passwordBytes > 128) {
    errors['creator.password'] = 'Passordet må være mellom 12 og 128 byte.'
  }
  const handicap = parseHandicap(creator.handicap)
  if (!handicap.ok) errors['creator.handicap'] = handicap.message
  return errors
}

export function validateAll(draft: WizardDraft, today: string): FieldErrors {
  return {
    ...validateTournament(draft.tournament, today),
    ...validateRounds(draft.rounds, draft.tournament),
    ...validateCreator(draft.creator),
  }
}

export function hasErrors(errors: FieldErrors): boolean {
  return Object.keys(errors).length > 0
}
