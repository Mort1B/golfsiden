import { decodeAuthSession } from './auth'
import { coordinateIdentity, identityChanged } from './identityQueue'
import { ApiHttpError, jsonRequest, requestDecoded, requestNoContent } from './http'

export async function readCookieSession() {
  try {
    return await requestDecoded('/api/auth/session', decodeAuthSession, { identityRequest: true })
  } catch (error) {
    if (error instanceof ApiHttpError && error.status === 401) return null
    throw error
  }
}

export const authRequests = {
  login: (username: string, password: string) => coordinateIdentity(() =>
    requestDecoded('/api/auth/login', decodeAuthSession, {
      ...jsonRequest('POST', { username, password }), identityRequest: true,
    }), session => session),
  logout: (originCsrf?: string) => coordinateIdentity(async () => {
    // The argument remains compatible with callers, but a queued logout must
    // clear the authoritative cookie even when its originating cache was null.
    void originCsrf
    const current = await readCookieSession()
    if (!current) return
    await requestNoContent('/api/auth/logout', {
      method: 'POST', headers: { 'x-csrf-token': current.csrf_token }, identityRequest: true,
    })
  }, () => null),
}

export async function requireCookieSession(csrfToken: string): Promise<void> {
  const current = await readCookieSession()
  if (!current || current.csrf_token !== csrfToken) throw identityChanged()
}
