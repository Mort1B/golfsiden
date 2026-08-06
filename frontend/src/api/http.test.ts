import { afterEach, describe, expect, it, vi } from 'vitest'
import { requestDecoded } from './http'

afterEach(() => vi.unstubAllGlobals())

describe('HTTP boundary', () => {
  it('preserves backend status, code, and message', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({
      error: { code: 'round_not_editable', message: 'locked' },
    }), { status: 409, headers: { 'content-type': 'application/json' } })))

    const result = requestDecoded('/test', () => 'unused')
    await expect(result).rejects.toMatchObject({
      status: 409,
      code: 'round_not_editable',
      message: 'locked',
    })
  })

  it('includes the session cookie for API requests', async () => {
    const fetchMock = vi.fn(async () => new Response(JSON.stringify({ ok: true }), {
      headers: { 'content-type': 'application/json' },
    }))
    vi.stubGlobal('fetch', fetchMock)

    await requestDecoded('/test', () => 'ok')

    expect(fetchMock).toHaveBeenCalledWith('/test', { credentials: 'include' })
  })
})
