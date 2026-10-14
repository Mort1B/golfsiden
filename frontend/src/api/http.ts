import { decodeObject } from './decoder'
import { identityChanged, identityVersion, settledIdentityVersion, waitForIdentity } from './identityQueue'

const apiUrl = import.meta.env.VITE_API_URL ?? ''

export interface ApiRequestInit extends RequestInit {
  /** Only the serialized cookie writer and its internal authority read bypass. */
  identityRequest?: boolean
}

const usesIdentity = (init?: ApiRequestInit): boolean => !init?.identityRequest && init?.credentials !== 'omit'

async function fetchResponse(path: string, init?: ApiRequestInit): Promise<Response> {
  const { identityRequest, ...request } = init ?? {}
  const origin = settledIdentityVersion()
  if (!identityRequest && request.credentials !== 'omit') {
    if (origin !== identityVersion()) await waitForIdentity()
    request.signal?.throwIfAborted()
    if (origin !== settledIdentityVersion()) throw identityChanged()
  }
  const response = await fetch(`${apiUrl}${path}`, { ...request, credentials: request.credentials ?? 'include' })
  if (!identityRequest && request.credentials !== 'omit' && origin !== identityVersion()) throw identityChanged()
  return response
}

export class ApiHttpError extends Error {
  readonly status: number
  readonly code: string

  constructor(status: number, code: string, message: string) {
    super(message)
    this.name = 'ApiHttpError'
    this.status = status
    this.code = code
  }
}

function errorDetails(value: unknown, status: number): { code: string; message: string } {
  if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
    const envelope = decodeObject(value, 'response')
    if (typeof envelope.error === 'object' && envelope.error !== null && !Array.isArray(envelope.error)) {
      const error = decodeObject(envelope.error, 'response.error')
      if (typeof error.code === 'string' && typeof error.message === 'string') {
        return { code: error.code, message: error.message }
      }
    }
  }
  return { code: 'http_error', message: `Forespørselen feilet (${status})` }
}

async function responseJson(path: string, init?: ApiRequestInit): Promise<unknown> {
  const origin = settledIdentityVersion()
  const response = await fetchResponse(path, init)
  const body: unknown = await response.json().catch(() => undefined)
  if (usesIdentity(init) && origin !== identityVersion()) throw identityChanged()
  if (!response.ok) {
    const details = errorDetails(body, response.status)
    throw new ApiHttpError(response.status, details.code, details.message)
  }
  return body
}

export async function requestNoContent(path: string, init?: ApiRequestInit): Promise<void> {
  const response = await fetchResponse(path, init)
  if (!response.ok) {
    const body: unknown = await response.json().catch(() => undefined)
    const details = errorDetails(body, response.status)
    throw new ApiHttpError(response.status, details.code, details.message)
  }
  if (response.status !== 204) throw new Error(`Ugyldig svar fra serveren (${path})`)
}

export async function requestDecoded<T>(
  path: string,
  decode: (value: unknown) => T,
  init?: ApiRequestInit,
): Promise<T> {
  return decode(await responseJson(path, init))
}

export async function requestUnchecked<T>(path: string, init?: ApiRequestInit): Promise<T> {
  return await responseJson(path, init) as T
}

export function jsonRequest(method: 'POST' | 'PUT' | 'PATCH', body: unknown, csrfToken?: string): RequestInit {
  const headers: Record<string, string> = { 'content-type': 'application/json' }
  if (csrfToken) headers['x-csrf-token'] = csrfToken
  return {
    method,
    headers,
    body: JSON.stringify(body),
  }
}

export function tournamentLiveUrl(tournamentId: string): string {
  return `${apiUrl}/api/tournaments/${tournamentId}/live`
}
