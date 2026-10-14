import { useEffect, useRef, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { fantasyKeys } from '../../api/fantasy'
import { useAuth } from '../auth/authContext'
import type { FantasyRecovery } from './fantasyDrafts'
import { openFantasyRound, submitFantasyLineup, validFantasyDraft } from './fantasyLineups'
import { fantasyRoundQuery } from './fantasyRoundQuery'
import type { useFantasyAction } from './useFantasy'

export interface FantasyBatchProgress { running: boolean; completed: string[]; total: number; message: string }
export function useFantasyBatch(tournament: string, records: React.RefObject<Record<string, FantasyRecovery>>,
  update: (round: string, patch: Partial<FantasyRecovery>) => void, action: ReturnType<typeof useFantasyAction>) {
  const { session } = useAuth(), client = useQueryClient(), busy = useRef(false), epoch = useRef(0)
  const [progress, setProgress] = useState<FantasyBatchProgress>({ running: false, completed: [], total: 0, message: '' })
  useEffect(() => client.getQueryCache().subscribe(event => {
    const key = event.query.queryKey, root = fantasyKeys.root(session?.user_id ?? '', tournament)
    if (root.every((part, i) => part === key[i]) && (event.type === 'removed' && event.query.getObserversCount() > 0
      || event.type === 'updated' && event.action.type === 'setState' && event.query.state.data === undefined)) epoch.current++
  }), [client, session?.user_id, tournament])
  // The caller supplies reviewed identifiers, never a retained authorized projection.
  const save = async (rounds: string[]) => {
    if (busy.current || action.pending || action.phase === 'refresh_failed' || !action.current() || !session || !rounds.length) return
    busy.current = true
    const runEpoch = epoch.current, completed: string[] = []
    let message = 'Alle gjennomgåtte utkast er lagret.'
    setProgress({ running: true, completed: [], total: rounds.length, message: 'Lagrer én runde om gangen …' })
    try {
      for (const round of rounds) {
        if (epoch.current !== runEpoch || !action.current() || !navigator.onLine) {
          message = 'Lagringen er satt på pause. Oppdater tilgang og gjennomgå resterende utkast.'; break
        }
        const record = records.current[round]
        if (!record?.draft || record.uncertain || record.rejected) {
          message = 'Avklar innsendingen og gjennomgå resterende utkast før du fortsetter.'; break
        }
        const options = fantasyRoundQuery(client, tournament, round, session)
        const view = await client.fetchQuery({ ...options, staleTime: 0 })
        const state = client.getQueryState(options.queryKey)
        if (epoch.current !== runEpoch || !action.current() || !state?.data || state.isInvalidated) {
          message = 'Tilgangen ble oppdatert. Kontroller resterende utkast før du fortsetter.'; break
        }
        const revision = view.selections.find(s => s.user_id === session.user_id)?.receipt?.revision ?? 0
        if (!openFantasyRound(view) || !validFantasyDraft(record.draft, view) || record.draft.revision !== revision) {
          message = 'En runde er stengt, har ugyldige valg eller et nyere lag. Kontroller utkastet før du fortsetter.'; break
        }
        const outcome = await submitFantasyLineup(tournament, round, { request_id: crypto.randomUUID(),
          expected_revision: revision, picks: [...record.draft.picks], captain: record.draft.captain }, false, action, update)
        if (outcome === 'acknowledged' || outcome === 'refresh_failed') completed.push(round)
        setProgress({ running: true, completed: [...completed], total: rounds.length, message: 'Lagrer én runde om gangen …' })
        if (outcome !== 'acknowledged') {
          message = 'Lagringen er satt på pause. Avklar innsendingen eller oppdater visningen før du lagrer resterende utkast.'; break
        }
      }
    } catch {
      message = 'Kunne ikke kontrollere neste runde. Oppdater og gjennomgå resterende utkast; ingen nye valg er sendt.'
    } finally {
      busy.current = false
      if (action.current()) setProgress({ running: false, completed: [...completed], total: rounds.length, message })
    }
  }
  return { ...progress, save, isRunning: () => busy.current }
}
