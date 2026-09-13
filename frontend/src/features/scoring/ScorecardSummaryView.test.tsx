// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import type { ScoringScorecard } from '../../api/scorecards'
import { ScorecardSummaryView } from './ScorecardSummaryView'

afterEach(cleanup)
function card(allocations: number[]): ScoringScorecard {
  return { projection: 'scoring', round_id: 'round', owner: { type: 'player', id: 'player' },
    number_of_holes: allocations.length, gross_total: 0, net_total: 0, holes_scored: 0,
    playing_handicap: allocations.reduce((a, b) => a + b, 0), complete: false,
    confirmed: false, confirmed_at: null, confirmed_by: null,
    holes: allocations.map((handicap_strokes, i) => ({ hole_id: `hole-${i}`, hole_number: i + 1,
      par: 4, stroke_index: allocations.length - i, handicap_strokes, net_strokes: null, score: null })) }
}
const props = { disabled: false, readOnly: false, confirming: false, confirmationError: null,
  confirmationRetryable: false, onHole: vi.fn(), onConfirm: vi.fn() }

it('shows server allocations on unscored holes beside metadata and preserves hole navigation', () => {
  render(<ScorecardSummaryView {...props} card={card([0, 1, 2, 3])} />)
  expect(screen.getByText('Ekstra slag fra spillehandicap')).toBeTruthy()
  expect(screen.queryByRole('img', { name: /hull 1$/ })).toBeNull()
  for (const [number, allocation] of [[2, 1], [3, 2], [4, 3]]) {
    const badge = screen.getByRole('img', { name: `${allocation} ekstra slag på hull ${number}` })
    expect(badge.textContent).toBe(`+${allocation}`)
    const button = badge.closest('button')
    if (!button) throw new Error('Missing hole button')
    expect(within(button).getByText('Netto –')).toBeTruthy()
    fireEvent.click(button)
    expect(props.onHole).toHaveBeenLastCalledWith(number)
  }
})

it('updates player/team owner allocations and explains signed strokes given back', () => {
  const view = render(<ScorecardSummaryView {...props} card={card([1, 2])} />)
  view.rerender(<ScorecardSummaryView {...props} card={{ ...card([-1, -2]), owner: { type: 'team', id: 'team' } }} />)
  expect(screen.queryByText('+1')).toBeNull()
  expect(screen.getByText('Slag som gis tilbake fra spillehandicap')).toBeTruthy()
  expect(screen.getByRole('img', { name: '2 slag gis tilbake på hull 2' }).textContent).toBe('−2')
  view.rerender(<ScorecardSummaryView {...props} card={card([0, 0])} />)
  expect(screen.queryAllByRole('img')).toHaveLength(0)
  expect(screen.queryByText(/fra spillehandicap/)).toBeNull()
})

it('only renders the visible allocation prefix in a restricted read summary', () => {
  const full = card(Array.from({ length: 18 }, () => 1))
  render(<ScorecardSummaryView {...props} readOnly card={{ projection: 'read', round_id: full.round_id,
    owner: full.owner, holes: full.holes.slice(0, 9), number_of_holes: 18, visible_hole_count: 9,
    playing_handicap: 18, gross_total: 0, net_total: 0, holes_scored: 0,
    complete: null, confirmed: null, confirmed_at: null, visibility: { mode: 'front_nine' } }} />)
  expect(screen.getAllByRole('img')).toHaveLength(9)
  expect(screen.queryByRole('button', { name: /Hull 10/ })).toBeNull()
  expect(screen.queryByRole('button', { name: /Bekreft/ })).toBeNull()
})

it('labels local values separately and waits for server net while preserving server totals', () => {
  render(<ScorecardSummaryView {...props} card={card([1, 1])} localScores={new Map([['hole-0', 5]])} />)
  expect(screen.getByText('Lokalt 5')).toBeTruthy()
  expect(screen.getByText('Netto venter')).toBeTruthy()
  expect(screen.getByText(/Summer og antall registrerte hull viser serverens scorekort/)).toBeTruthy()
  expect(screen.getAllByRole('button')).toHaveLength(2)
})
