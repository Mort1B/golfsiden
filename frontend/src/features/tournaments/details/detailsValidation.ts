import { decodeDate } from '../../../api/decoder'
import type { Round, Tournament } from '../../../api/types'
export type DetailsFields = Pick<Tournament, 'name' | 'description' | 'start_date' | 'end_date'>
export function detailsFields(tournament: Tournament): DetailsFields {
  return { name: tournament.name, description: tournament.description, start_date: tournament.start_date, end_date: tournament.end_date }
}
export function detailsErrors(fields: DetailsFields, rounds: Round[]): Partial<Record<keyof DetailsFields, string>> {
  const errors: Partial<Record<keyof DetailsFields, string>> = {}, encoder = new TextEncoder()
  if (!fields.name.trim() || fields.name.includes('\0')) errors.name = 'Skriv et gyldig turneringsnavn.'
  else if (encoder.encode(fields.name).length > 120) errors.name = 'Turneringsnavnet er for langt. Velg et kortere navn.'
  if (fields.description.includes('\0') || encoder.encode(fields.description).length > 2000) errors.description = 'Beskrivelsen er for lang. Forkort teksten og prøv igjen.'
  for (const key of ['start_date', 'end_date'] as const) {
    try { decodeDate(fields[key], key) } catch { errors[key] = 'Velg en gyldig dato.' }
  }
  if (!errors.start_date && !errors.end_date) {
    if (fields.end_date < fields.start_date) errors.end_date = 'Sluttdato kan ikke være før startdato.'
    else if (rounds.some(round => round.round_date < fields.start_date || round.round_date > fields.end_date)) errors.end_date = 'Perioden må inneholde datoene til alle rundene. Rundedatoer flyttes ikke.'
  }
  return errors
}
