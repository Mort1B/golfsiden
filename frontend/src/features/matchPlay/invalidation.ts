import type { QueryClient } from '@tanstack/react-query'
export function invalidateMatchQueries(client: QueryClient, user: string, round: string, tournament: string): Promise<void> {
  return client.invalidateQueries({ predicate: query => {
    const key = query.queryKey
    if (key[0] !== 'private-workspace' || key[1] !== user) return false
    return key[2] === 'rounds' && key[3] === round && (key[4] === 'match-play' || key[4] === 'pairing-validation')
      || key[2] === 'tournaments' && key[3] === tournament && key[4] === 'match-table'
  } })
}
