import type { QueryClient } from '@tanstack/react-query'

export const privateWorkspaceKeys = {
  root: ['private-workspace'] as const,
  user: (userId: string) => ['private-workspace', userId] as const,
  completion: (userId: string, roundId: string) =>
    ['private-workspace', userId, 'rounds', roundId, 'completion-validation'] as const,
  scoreAccess: (userId: string, roundId: string) =>
    ['private-workspace', userId, 'rounds', roundId, 'score-access'] as const,
  invitations: (userId: string, tournamentId: string) =>
    ['private-workspace', userId, 'tournaments', tournamentId, 'invitations'] as const,
}

export function clearPrivateWorkspace(queryClient: QueryClient): void {
  queryClient.removeQueries({ queryKey: privateWorkspaceKeys.root })
}
