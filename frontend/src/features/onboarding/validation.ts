import type { CreatorDraft, RoundDraft, TournamentDraft, WizardDraft } from './wizardState'

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

function basicEmailValid(value: string): boolean {
  const email = value.trim()
  const hasWhitespaceOrControl = Array.from(email).some((character) => {
    const codePoint = character.codePointAt(0) ?? 0
    return character.trim() === '' || codePoint < 32 || codePoint === 127
  })
  if (!email || byteLength(email) > 254 || hasWhitespaceOrControl) return false
  const separator = email.indexOf('@')
  if (separator <= 0 || separator !== email.lastIndexOf('@')) return false
  const domain = email.slice(separator + 1)
  return Boolean(domain) && !domain.startsWith('.') && !domain.endsWith('.')
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
    if (round.scoringFormat !== 'individual_stroke_play' && round.scoringFormat !== 'team_scramble') {
      errors[`${prefix}.scoringFormat`] = 'Velg en støttet spilleform.'
    }
  }
  return errors
}

export function validateCreator(creator: CreatorDraft): FieldErrors {
  const errors: FieldErrors = {}
  const nameError = validateName(creator.displayName, 'Visningsnavn')
  if (nameError) errors['creator.displayName'] = nameError
  if (!basicEmailValid(creator.email)) errors['creator.email'] = 'Skriv inn en gyldig e-postadresse.'
  const passwordBytes = byteLength(creator.password)
  if (passwordBytes < 12 || passwordBytes > 128) {
    errors['creator.password'] = 'Passordet må være mellom 12 og 128 byte.'
  }
  const handicap = Number(creator.handicap)
  if (!creator.handicap.trim() || !Number.isFinite(handicap) || handicap < -10 || handicap > 54) {
    errors['creator.handicap'] = 'Handicap må være et tall mellom −10,0 og 54,0.'
  }
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
