// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { fourBallFixture } from '../../../api/fourBall/fixtures'
import { decodeFourBallRead } from '../../../api/fourBall/decoders'
import { FourBallSummary } from './FourBallSummary'
import { FourBallReadView } from './FourBallReadView'
import { PartnerInput } from './PartnerInput'
afterEach(cleanup)
it('shows each named partner allocation and separate gross/net attribution without a team handicap', () => {
  const card = fourBallFixture(true)
  render(<FourBallSummary card={card} />)
  expect(screen.getAllByText(card.partners[0].display_name).length).toBeGreaterThan(0)
  expect(screen.getAllByRole('img', { name: /ekstra slag på hull/ })).toHaveLength(18)
  expect(screen.queryByText('Spille-HCP')).toBeNull()
  expect(screen.getAllByText('Ikke registrert')).toHaveLength(18)
})
it('keeps retry and discard usable when queue storage failure disables ordinary edits', () => {
  const discard = vi.fn(), retry = vi.fn()
  render(<PartnerInput partner={fourBallFixture().partners[0]} hole={1} par={4} strokes={0} disabled sync={{ desired: { type: 'no_score' },
    server: null, navigationLocked: true, storageReady: false, phase: 'failed', error: 'Lagringen er utilgjengelig',
    setInput: vi.fn(), retry, discard }} />)
  expect(screen.getByRole('button', { name: 'Registrer par (4)' }).hasAttribute('disabled')).toBe(true)
  fireEvent.click(screen.getByRole('button', { name: 'Prøv lagring igjen' }))
  fireEvent.click(screen.getByRole('button', { name: 'Forkast ulagret endring' }))
  expect(retry).toHaveBeenCalledOnce(); expect(discard).toHaveBeenCalledOnce()
})
it('requires explicit pickup confirmation and displays server separately from pending intent', () => {
  const setInput = vi.fn(), card = fourBallFixture()
  render(<PartnerInput partner={card.partners[0]} hole={1} par={4} strokes={1} disabled={false} sync={{ desired: { type: 'numeric', gross_strokes: 5 },
    server: null, navigationLocked: false, storageReady: true, phase: 'queued', error: null, setInput, retry: vi.fn(), discard: vi.fn() }} />)
  fireEvent.click(screen.getByRole('button', { name: 'Plukket opp' }))
  expect(setInput).not.toHaveBeenCalled()
  fireEvent.click(screen.getByRole('button', { name: 'Ja, plukket opp' }))
  expect(setInput).toHaveBeenCalledWith({ type: 'no_score' })
  expect(screen.getByText('Server: Ikke registrert')).toBeTruthy()
  expect(screen.getByText(/Lokalt: 5 slag/)).toBeTruthy()
})
it('read-only hole navigation shows the selected hole and returns to summary', () => {
  const source = fourBallFixture()
  const card = decodeFourBallRead(source, source.round_id, source.owner.id)
  const onHole = vi.fn()
  const view = render(<FourBallReadView card={card} view="hole" holeNumber={2} onHole={onHole} />)
  expect(screen.getByRole('heading', { name: 'Hull 2 · par 4 · indeks 2' })).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: 'Neste hull' }))
  expect(onHole).toHaveBeenCalledWith(3)
  view.rerender(<FourBallReadView card={card} view="summary" holeNumber={2} onHole={onHole} />)
  expect(screen.getAllByRole('button', { name: /Hull .*par/ })).toHaveLength(18)
})
