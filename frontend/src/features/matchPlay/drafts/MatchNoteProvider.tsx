import { useBlocker, useMatch } from 'react-router-dom'
import { useEffect, useState, type ReactNode } from 'react'
import { useAuth } from '../../auth/authContext'
import { useScoringGuard } from '../../scoring/scoringGuardContext'
import { MatchNoteContext, useMatchNoteDrafts } from './context'
import { MatchNoteStore } from './store'
export function MatchNoteProvider({ children }: { children: ReactNode }) {
  const account = useAuth().session?.user_id ?? ''
  return <AccountNotes key={account} account={account}>{children}</AccountNotes>
}
function AccountNotes({ account, children }: { account: string; children: ReactNode }) {
  const [store] = useState(() => new MatchNoteStore(account))
  // Account-keyed lifetime is separate from either CSRF-keyed delivery runtime.
  useEffect(() => { store.activate(); return store.close }, [store])
  return <MatchNoteContext value={store}><NoteGuard />{children}</MatchNoteContext>
}
function NoteGuard() {
  const { drafts } = useMatchNoteDrafts(), { setBlocked, blocked: scoringBlocked } = useScoringGuard()
  const matchRoute = useMatch('/rounds/:roundId/matches/:matchId/score')
  const blocked = drafts.length > 0
  useEffect(() => { setBlocked(blocked); return () => setBlocked(false) }, [blocked, setBlocked])
  useEffect(() => {
    if (!blocked) return
    const warn = (event: BeforeUnloadEvent) => { event.preventDefault(); event.returnValue = '' }
    window.addEventListener('beforeunload', warn)
    return () => window.removeEventListener('beforeunload', warn)
  }, [blocked])
  return matchRoute !== null ? <MatchNavigationGuard blocked={blocked || scoringBlocked} /> : null
}
function MatchNavigationGuard({ blocked }: { blocked: boolean }) {
  const blocker = useBlocker(blocked)
  useEffect(() => { if (blocker.state === 'blocked') blocker.reset() }, [blocker])
  return null
}
