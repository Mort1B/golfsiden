export function safeReturnTo(value: string | null): string {
  if (!value?.startsWith('/') || value.includes('\\')) return '/score'
  const base = new URL('https://guttas-golf.invalid')
  const resolved = new URL(value, base)
  return resolved.origin === base.origin ? value : '/score'
}

export function signInTarget(pathname: string, search: string, hash: string): string {
  return `/login?returnTo=${encodeURIComponent(`${pathname}${search}${hash}`)}`
}
