import { useCallback, useState, type ReactNode } from 'react'
import { useAuth } from '../auth/authContext'
import { ownerEquals } from '../../api/scorecards'
import { ScoreResumeContext, type ScoreResumeSelection } from './scoreResumeContext'

export function ScoreResumeProvider({ children }: { children: ReactNode }) {
  const { session } = useAuth()
  const userId = session?.user_id ?? null
  const [state, setState] = useState<{ userId: string | null; selection: ScoreResumeSelection | null }>({ userId, selection: null })
  // Clear before descendants render for another identity, without remounting the
  // router: creator onboarding keeps a one-time receipt across session creation.
  if (state.userId !== userId) setState({ userId, selection: null })
  const remember = useCallback((next: ScoreResumeSelection) => {
    setState((previous) => {
      if (previous.userId !== userId) return previous
      const selected = previous.selection
      return selected?.tournamentId === next.tournamentId && selected.roundId === next.roundId
        && ownerEquals(selected.owner, next.owner) ? previous : { userId, selection: next }
    })
  }, [userId])
  return <ScoreResumeContext.Provider value={{ selection: state.userId === userId ? state.selection : null, remember }}>{children}</ScoreResumeContext.Provider>
}
