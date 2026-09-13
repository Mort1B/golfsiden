import { ApiHttpError } from '../../api/http'

export const invalidRecoveryMessage = 'Lenken er ugyldig, utløpt eller allerede brukt. Be arrangøren om en ny lenke.'
export function recoveryMessage(error: unknown): string {
  if (error instanceof ApiHttpError) {
    if (error.code === 'password_recovery_invalid') return invalidRecoveryMessage
    if (error.code === 'current_password_incorrect') return 'Ditt nåværende passord er ikke riktig. Prøv igjen.'
    if (error.status === 401) return 'Økten er utløpt. Logg inn igjen.'
    if (error.status === 403) return 'Denne handlingen er ikke tillatt. Administratorkontoer og spillere uten en kvalifisert arrangør må få hjelp fra nettstedsoperatøren.'
    if (error.status === 429) return 'For mange forsøk. Vent litt før du prøver igjen.'
    if (error.status === 400) return 'Kontroller passordfeltene og prøv igjen.'
    if (error.status === 503) return 'Passordhjelpen er ikke tilgjengelig nå. Kontakt nettstedsoperatøren.'
  }
  return 'Vi fikk ikke bekreftet handlingen. Prøv igjen. En ny lenke erstatter tidligere lenker.'
}
export function recoveryExpiry(iso: string): string {
  return new Intl.DateTimeFormat('nb-NO', { dateStyle: 'short', timeStyle: 'short' }).format(new Date(iso))
}
