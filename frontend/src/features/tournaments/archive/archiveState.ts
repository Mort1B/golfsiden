import { ApiHttpError } from '../../../api/http'

export function archiveFailure(error: unknown): string {
  if (error instanceof ApiHttpError) {
    if (error.status === 401) return 'Økten er utløpt. Logg inn på nytt.'
    if (error.status === 403) return 'Du har ikke lenger administratortilgang til turneringen.'
    if (error.status === 404) return 'Turneringen finnes ikke lenger.'
    if (error.code === 'tournament_archive_invalid_state') return 'Turneringen må være fullført før arkivering. Status oppdateres.'
    if (error.status === 409) return 'Turneringen er endret. Se oppdatert status og kontroller på nytt.'
  }
  return 'Svaret kunne ikke bekreftes. Kontroller oppdatert turneringsstatus før du prøver igjen.'
}
