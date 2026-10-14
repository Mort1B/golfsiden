// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { authKeys } from '../../api/auth'
import { api } from '../../api/client'
import * as onboarding from '../../api/onboarding'
import * as invitations from '../../api/invitations'
import { waitForIdentity } from '../../api/identityQueue'
import { tournamentKeys } from '../../api/tournaments'
import { AuthContext, type AuthContextValue } from './authContext'
import { publishSessionTransition } from './sessionTransition'
import { JoinExperience } from '../invitations/JoinExperience'
import { useOnboardingSubmission } from '../onboarding/useOnboardingSubmission'
import { createInitialDraft } from '../onboarding/wizardState'
import { round, session, tournament } from '../tournaments/lifecycle/__tests__/fixtures'

function deferred<T>() {
  let resolve: (value: T) => void = () => { throw new Error('Not initialized') }
  const promise = new Promise<T>(done => { resolve = done })
  return { promise, resolve }
}
const created: onboarding.OnboardingResponse = {
  tournament, rounds: [round], session,
  creator: { user_id: session.user_id, player_id: session.player_id ?? '', tournament_role: 'admin' },
  invitation: { id: round.id, token: 'a'.repeat(43), expires_at: session.expires_at, max_uses: null },
}
const joined: invitations.InvitationRegistrationResult = {
  status: 'joined', tournament_id: tournament.id, player_id: session.player_id ?? '', session,
}
let client: QueryClient
let auth: AuthContextValue
const onCreated = vi.fn()
function Creation() {
  const submission = useOnboardingSubmission(onCreated)
  const draft = createInitialDraft('2026-10-14')
  draft.creator.handicap = '12'
  return <><button onClick={() => void submission.submit(draft)}>Create</button><output>{submission.state.status}</output></>
}
function tree(child: React.ReactNode) {
  return render(<QueryClientProvider client={client}><AuthContext value={auth}><MemoryRouter>{child}</MemoryRouter></AuthContext></QueryClientProvider>)
}
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  client.setQueryData(authKeys.session, null)
  auth = { session: null, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), retry: vi.fn(),
    establishSession: vi.fn(value => publishSessionTransition(client, value)) }
  onCreated.mockClear()
})
afterEach(async () => { cleanup(); client.clear(); await waitForIdentity(); vi.restoreAllMocks(); vi.unstubAllGlobals() })

it('keeps the ordinary onboarding receipt and publishes it after its owner acknowledges creation', async () => {
  vi.spyOn(onboarding, 'createOnboarding').mockResolvedValue(created)
  tree(<Creation />); fireEvent.click(screen.getByText('Create'))
  await screen.findByText('success')
  expect(onCreated).toHaveBeenCalledTimes(1)
  expect(onCreated.mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(auth.establishSession).mock.invocationCallOrder[0] ?? 0)
  expect(client.getQueryData(tournamentKeys.rounds(session.user_id, tournament.id))).toEqual([round])
})

it.each(['unmount', 'newer-intent', 'renewed-session'])('does not publish a departed onboarding receipt after %s', async mode => {
  const pending = deferred<onboarding.OnboardingResponse>()
  vi.spyOn(onboarding, 'createOnboarding').mockReturnValue(pending.promise)
  const view = tree(<Creation />); fireEvent.click(screen.getByText('Create'))
  await waitFor(() => expect(onboarding.createOnboarding).toHaveBeenCalled())
  let login: Promise<unknown> | undefined
  if (mode === 'unmount') view.unmount()
  if (mode === 'renewed-session') publishSessionTransition(client, { ...session, csrf_token: 'renewed' })
  if (mode === 'newer-intent') {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify(session))))
    login = api.login('newer', 'password')
  }
  await act(async () => { pending.resolve(created); await login })
  expect(auth.establishSession).not.toHaveBeenCalled()
  expect(onCreated).not.toHaveBeenCalled()
  expect(client.getQueryData(tournamentKeys.rounds(session.user_id, tournament.id))).toBeUndefined()
})

async function register() {
  const section = await screen.findByRole('heading', { name: 'Ny spiller' })
  const form = section.parentElement
  if (!form) throw new Error('Missing registration section')
  const fields = within(form)
  for (const [label, value] of [['Visningsnavn', 'New Player'], ['Brukernavn', 'new_player'], ['Passord', 'a long password'], ['Handicapindeks', '12']]) {
    if (label && value) fireEvent.change(fields.getByLabelText(new RegExp(`^${label}`)), { target: { value } })
  }
  fireEvent.click(fields.getByRole('button', { name: 'Opprett konto og bli med' }))
}

it.each(['ordinary', 'unmount', 'newer-intent', 'refetch-error'])('fences invitation registration continuation: %s', async mode => {
  window.history.replaceState(null, '', `/#token=${'a'.repeat(43)}`)
  vi.spyOn(invitations, 'previewInvitation').mockResolvedValue({ tournament,
    invitation: { expires_at: session.expires_at } })
  const pending = deferred<invitations.InvitationRegistrationResult>()
  vi.spyOn(invitations, 'registerInvitation').mockReturnValue(pending.promise)
  const view = tree(<JoinExperience invitationId={round.id} />)
  await register(); await waitFor(() => expect(invitations.registerInvitation).toHaveBeenCalled())
  if (mode === 'unmount') view.unmount()
  if (mode === 'refetch-error') {
    vi.mocked(invitations.previewInvitation).mockRejectedValue(new TypeError('refocused preview unavailable'))
    await act(async () => { await client.refetchQueries({ queryKey: invitations.invitationKeys.preview(round.id) }) })
  }
  let login: Promise<unknown> | undefined
  if (mode === 'newer-intent') {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify(session))))
    login = api.login('newer', 'password')
  }
  await act(async () => { pending.resolve(joined); await login })
  if (mode === 'ordinary' || mode === 'refetch-error') {
    await screen.findByRole('heading', { name: 'Du er med!' })
    expect(auth.establishSession).toHaveBeenCalledTimes(1)
    expect(client.getQueryData(authKeys.session)).toEqual(session)
  } else {
    expect(auth.establishSession).not.toHaveBeenCalled()
    expect(client.getQueryData(authKeys.session)).toBeNull()
    expect(screen.queryByRole('heading', { name: 'Du er med!' })).toBeNull()
  }
})
