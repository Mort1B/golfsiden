import { ApiHttpError } from '../../api/http'

export type PreviewFailure =
  | 'invalid'
  | 'expired'
  | 'revoked'
  | 'exhausted'
  | 'closed'
  | 'retryable'

export function previewFailure(error: unknown): PreviewFailure {
  if (!(error instanceof ApiHttpError)) return 'retryable'
  if (error.code === 'invitation_invalid') return 'invalid'
  if (error.code === 'invitation_expired') return 'expired'
  if (error.code === 'invitation_revoked') return 'revoked'
  if (error.code === 'invitation_exhausted') return 'exhausted'
  if (error.code === 'tournament_not_joinable') return 'closed'
  return 'retryable'
}

export function joinErrorMessage(error: unknown): string {
  if (error instanceof ApiHttpError) {
    if (error.code === 'username_already_registered') {
      return 'Brukernavnet er allerede tatt. Velg et annet brukernavn eller logg inn.'
    }
    if (error.code === 'already_authenticated') {
      return 'Du er allerede logget inn. Bruk den eksisterende kontoen for å bli med.'
    }
    if (error.code === 'account_player_required') {
      return 'Kontoen din er ikke koblet til en spillerprofil. Be en turneringsadministrator om hjelp.'
    }
    if (error.code === 'player_inactive') return 'Spillerprofilen din er deaktivert.'
    if (error.code === 'player_withdrawn') return 'Spilleren din er trukket fra denne turneringen.'
    if (error.code === 'invitation_expired') return 'Invitasjonen har utløpt.'
    if (error.code === 'invitation_revoked') return 'Invitasjonen er trukket tilbake.'
    if (error.code === 'invitation_exhausted') return 'Invitasjonen har nådd maks antall påmeldinger.'
    if (error.code === 'tournament_not_joinable') return 'Turneringen er ikke åpen for påmelding.'
    if (error.code === 'invitation_invalid') return 'Invitasjonslenken er ugyldig.'
  }
  return error instanceof Error ? error.message : 'Kunne ikke fullføre påmeldingen. Prøv igjen.'
}
