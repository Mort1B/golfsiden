// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { afterEach, expect, it } from 'vitest'
import { tieBoard, tieRounds } from '../../api/leaderboards/__tests__/tieBreakFixtures'
import { TournamentStandings } from './TournamentStandings'

afterEach(cleanup)
it.each(['gross', 'net'] as const)('explains evaluated %s final scores and retains server positions', (metric) => {
  const board = tieBoard(metric)
  render(<MemoryRouter><TournamentStandings leaderboard={board} rounds={tieRounds} /></MemoryRouter>)
  expect(screen.getByText(/også når runden ikke teller blant de beste/)).toBeTruthy()
  expect(screen.getAllByText(/sammenlignet ved lik totalscore/)).toHaveLength(3)
  expect(screen.getByText(`Siste runde: +${metric === 'gross' ? 2 : 4} ${metric === 'gross' ? 'brutto' : 'netto'} · sammenlignet ved lik totalscore`)).toBeTruthy()
  expect(screen.getAllByRole('link').map((link) => link.querySelector('.leaderboard-position')?.textContent)).toEqual(metric === 'gross' ? ['1', 'T2', 'T2'] : ['T1', 'T1', '3'])
})
it('does not imply evaluation for shared or missing comparisons', () => {
  const board = tieBoard('gross', 'shared_positions')
  board.tie_break_policy = 'final_round_score'
  render(<MemoryRouter><TournamentStandings leaderboard={board} rounds={tieRounds} /></MemoryRouter>)
  expect(screen.queryByText(/sammenlignet ved lik totalscore/)).toBeNull()
  expect(screen.getAllByRole('link').every(link => link.querySelector('.leaderboard-position')?.textContent === 'T1')).toBe(true)
})
