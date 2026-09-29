import { useAuth } from '../../auth/authContext'
import { preparedKeys, isTerminal, type PreparedSelection } from './eligibility'
import { useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect, useSyncExternalStore } from 'react'
import { useNavigate } from 'react-router-dom'
import { scoringSearch } from '../selection'
import { usePreparedScore } from './context'
import { useOnline } from './useOnline'
export function PreparedReturnOffer() {
  const online = useOnline(), { prepared, available } = usePreparedScore(), navigate = useNavigate()
  const client = useQueryClient(), { session } = useAuth()
  const subscribe = useCallback((notify: () => void) => client.getQueryCache().subscribe(notify), [client])
  const failed = useSyncExternalStore(subscribe, () => !!prepared && !!session && preparedKeys(session.user_id, prepared).some(key => {
    const state = client.getQueryState(key); return state?.fetchStatus === 'paused' || !!state?.error && !isTerminal(state.error)
  }))
  if (online && !failed) return null
  return <section className="prepared-return" aria-label="Score uten forbindelse">
    {prepared ? <><p>Du er uten forbindelse. Du kan gå tilbake til kortet du allerede har åpnet, på hull {prepared.holeNumber}. Tilgang og rundestatus må kontrolleres når forbindelsen er tilbake.</p>
      <button type="button" onClick={() => {
        const target = available(); if (!target) return
        const search = scoringSearch({ ...target, view: 'hole' }); search.set('prepared', '1')
        void navigate(`/score?${search}`)
      }}>Tilbake til åpnet scorekort</button></>
      : <p>Ingen klargjorte scorekort er tilgjengelige. Koble til nettet og åpne kortet før du fører score uten forbindelse.</p>}
  </section>
}

export function PrepareScoreVisit({ target, ready }: { target: PreparedSelection; ready: boolean }) {
  const { prepare } = usePreparedScore()
  const { tournamentId, roundId, owner, holeNumber } = target
  useEffect(() => { if (ready) prepare({ tournamentId, roundId, owner, holeNumber }) }, [prepare, ready, tournamentId, roundId, owner, holeNumber])
  return null
}
