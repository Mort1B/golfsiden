import { ApiHttpError } from '../../api/http'
export const invalidClaimMessage = 'Lenken er ugyldig, utløpt eller allerede brukt. Har du allerede opprettet kontoen, bruk vanlig innlogging eller passordhjelp. Kontakt arrangøren hvis lenken utløp før du opprettet konto.'
export function claimMessage(error: unknown): string {
  if (error instanceof ApiHttpError) {
    switch (error.code) {
      case 'claim_invalid': case 'claim_unavailable': return invalidClaimMessage
      case 'player_has_account': return 'Spilleren har allerede en konto. Bruk passordhjelp ved behov.'
      case 'player_withdrawn': return 'Spilleren er trukket fra turneringen.'
      case 'tournament_not_joinable': return 'Spillere kan ikke opprettes eller fjernes når turneringen er avsluttet eller arkivert.'
      case 'player_is_admin': return 'En administrator kan ikke fjernes fra turneringen.'
      case 'player_assigned_draft': return 'Fjern spilleren fra lag, spillegrupper og matcher i kladder først.'
      case 'player_round_in_progress': return 'Spilleren deltar i en åpen eller fullført runde. Lås runden før spilleren fjernes.'
      case 'player_participation_changed': return 'Deltakelsen er endret. Oppdater listen og prøv igjen.'
      case 'username_already_registered': return 'Brukernavnet er opptatt. Velg et annet.'
      case 'already_authenticated': return 'Du er allerede innlogget. Logg ut før du tar i bruk denne kontoen.'
    }
    if (error.status === 401) return 'Økten er utløpt. Logg inn igjen.'
    if (error.status === 403) return 'Du har ikke tilgang til denne handlingen.'
    if (error.status === 429) return 'For mange forsøk. Vent litt og prøv igjen.'
    if (error.status === 400) return 'Kontroller feltene og prøv igjen.'
  }
  return 'Handlingen kunne ikke bekreftes. Oppdater før du prøver igjen.'
}
