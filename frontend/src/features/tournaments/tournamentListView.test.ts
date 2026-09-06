import { expect, it } from 'vitest'
import { matchesTournamentView, tournamentListView, tournamentViewSearch } from './tournamentListView'
import { tournament } from './lifecycle/__tests__/fixtures'

it('defaults absent or invalid views to current and preserves unrelated URL fields', () => {
  expect(tournamentListView(new URLSearchParams())).toBe('current')
  expect(tournamentListView(new URLSearchParams('view=unknown'))).toBe('current')
  expect(tournamentListView(new URLSearchParams('view=archived'))).toBe('archived')
  expect(tournamentViewSearch(new URLSearchParams('view=all&keep=yes'), 'current')).toBe('?keep=yes')
  expect(tournamentViewSearch(new URLSearchParams(), 'archived')).toBe('?view=archived')
})
it.each(['draft', 'active', 'completed', 'archived'] as const)('partitions %s without treating completed as archived', (status) => {
  const item = { ...tournament, status }
  expect(matchesTournamentView(item, 'current')).toBe(status !== 'archived')
  expect(matchesTournamentView(item, 'archived')).toBe(status === 'archived')
  expect(matchesTournamentView(item, 'all')).toBe(true)
})
