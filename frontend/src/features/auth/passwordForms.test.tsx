// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { createRef } from 'react'
import { RegistrationForm } from '../invitations/JoinForms'
import { CreatorStep } from '../onboarding/CreatorStep'
import { validateCreator } from '../onboarding/validation'
import { PASSWORD_GUIDANCE } from './password'

afterEach(cleanup)

it('validates invitation passwords by bytes before submitting, including multibyte minimum', async () => {
  const submit = vi.fn().mockResolvedValue(undefined)
  render(<RegistrationForm disabled={false} error={null} onSubmit={submit} />)
  fireEvent.change(screen.getByLabelText('Visningsnavn'), { target: { value: 'Spiller' } })
  fireEvent.change(screen.getByLabelText(/^Brukernavn/), { target: { value: 'spiller' } })
  fireEvent.change(screen.getByLabelText(/^Handicapindeks/), { target: { value: '14,4' } })
  const password = screen.getByLabelText(/^Passord/)
  const form = password.closest('form')
  if (!form) throw new Error('Missing registration form')
  expect(password.getAttribute('minlength')).toBeNull()
  expect(password.getAttribute('maxlength')).toBeNull()
  expect(screen.getByText(PASSWORD_GUIDANCE)).toBeTruthy()
  for (const [input, message] of [['ø'.repeat(5), 'for kort'], ['🏌'.repeat(33), 'for langt']] as const) {
    fireEvent.change(password, { target: { value: input } })
    fireEvent.submit(form)
    expect(await screen.findByRole('alert')).toHaveProperty('textContent', expect.stringContaining(message))
    expect(submit).not.toHaveBeenCalled()
  }
  fireEvent.change(password, { target: { value: 'ø'.repeat(6) } })
  fireEvent.submit(form)
  await waitFor(() => expect(submit).toHaveBeenCalledWith({ account: { username: 'spiller', password: 'ø'.repeat(6) }, player: { display_name: 'Spiller', handicap_index: 14.4 } }))
})

it('uses the same guidance and byte boundaries in creator onboarding', () => {
  const creator = { displayName: 'Spiller', username: 'spiller', password: 'ø'.repeat(6), handicap: '14,4' }
  render(<CreatorStep value={creator} errors={{}} onChange={vi.fn()} onBack={vi.fn()} onNext={vi.fn()} headingRef={createRef<HTMLHeadingElement>()} />)
  expect(screen.getByLabelText(/^Passord/).getAttribute('minlength')).toBeNull()
  expect(screen.getByText(PASSWORD_GUIDANCE)).toBeTruthy()
  expect(validateCreator(creator)).toEqual({})
  expect(validateCreator({ ...creator, password: 'ø'.repeat(5) })['creator.password']).toContain('for kort')
  expect(validateCreator({ ...creator, password: '🏌'.repeat(33) })['creator.password']).toContain('for langt')
})
