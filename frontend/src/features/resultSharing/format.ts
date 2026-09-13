const date = new Intl.DateTimeFormat('nb-NO', { dateStyle: 'medium', timeStyle: 'short' })
const time = new Intl.DateTimeFormat('nb-NO', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
export const shareExpiry = (value: string): string => date.format(new Date(value))
export const shareUpdated = (value: number): string => time.format(new Date(value))
export function shareToken(fragment: string): string | null {
  return /^#token=[A-Za-z0-9_-]{43}$/.test(fragment) ? fragment.slice(7) : null
}
export function sharedResultsUrl(grantId: string, token: string): string {
  const url = new URL(`/results/shared/${encodeURIComponent(grantId)}`, window.location.origin)
  url.hash = new URLSearchParams({ token }).toString()
  return url.href
}
