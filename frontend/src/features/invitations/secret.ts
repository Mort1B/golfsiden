export type InvitationSecret =
  | { status: 'missing' }
  | { status: 'malformed' }
  | { status: 'valid'; token: string }

const fragmentPattern = /^#token=([A-Za-z0-9_-]{43})$/

export function parseInvitationSecret(fragment: string): InvitationSecret {
  if (fragment === '' || fragment === '#') return { status: 'missing' }
  const match = fragmentPattern.exec(fragment)
  const token = match?.[1]
  return token ? { status: 'valid', token } : { status: 'malformed' }
}

export function clearInvitationFragment(): void {
  window.history.replaceState(window.history.state, '', `${window.location.pathname}${window.location.search}`)
}
