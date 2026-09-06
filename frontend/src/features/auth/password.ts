export const PASSWORD_GUIDANCE = 'Bruk et langt passord, gjerne en setning. Mellomrom beholdes.'

export function passwordValidationMessage(password: string): string | null {
  const bytes = new TextEncoder().encode(password).length
  if (bytes < 12) return 'Passordet er for kort. Legg til flere tegn eller ord. Det må bruke minst 12 byte; vanlige bokstaver og tall bruker én byte hver.'
  if (bytes > 128) return 'Passordet er for langt. Fjern noen tegn eller ord. Grensen er 128 byte; æ, ø, å og emoji bruker flere byte per tegn.'
  return null
}
