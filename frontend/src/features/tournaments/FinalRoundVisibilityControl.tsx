import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { CheckCircle2, Eye, EyeOff, LoaderCircle, RefreshCw } from 'lucide-react'
import {
  finalRoundVisibilityApi,
  finalRoundVisibilityKeys,
  type FinalRoundVisibility,
} from '../../api/finalRoundVisibility'
import { handleTournamentLiveSignal } from '../../api/liveInvalidation'
import type { Round, Tournament } from '../../api/types'
import { useAuth } from '../auth/authContext'
import { finalRoundVisibilityFailure } from './finalRoundVisibility'

interface Props {
  tournament: Tournament
  finalRound: Round
}

export function FinalRoundVisibilityControl({ tournament, finalRound }: Props) {
  const auth = useAuth()
  const queryClient = useQueryClient()
  const userId = auth.session?.user_id ?? ''
  const queryKey = finalRoundVisibilityKeys.detail(userId, tournament.id)
  const [receipt, setReceipt] = useState<string | null>(null)
  const visibilityQuery = useQuery({
    queryKey,
    queryFn: () => finalRoundVisibilityApi.get(tournament.id),
    enabled: userId.length > 0,
  })
  const mutation = useMutation({
    mutationFn: (backNineHidden: boolean) => {
      const csrfToken = auth.session?.csrf_token
      const current = visibilityQuery.data
      if (!csrfToken) throw new Error('Økten mangler. Logg inn på nytt.')
      if (!current) throw new Error('Synlighetsstatusen må lastes før den kan endres.')
      return finalRoundVisibilityApi.update(tournament.id, {
        back_nine_hidden: backNineHidden,
        expected_visibility_updated_at: current.visibility_updated_at,
      }, csrfToken)
    },
  })

  const save = async (backNineHidden: boolean) => {
    if (mutation.isPending) return
    mutation.reset()
    setReceipt(null)
    try {
      const saved = await mutation.mutateAsync(backNineHidden)
      queryClient.setQueryData<FinalRoundVisibility>(queryKey, saved)
      setReceipt(backNineHidden
        ? 'Hull 10–18 er skjult igjen. Serverstatusen er bekreftet.'
        : 'Hull 10–18 er frigitt. Serverstatusen er bekreftet.')
      await handleTournamentLiveSignal(queryClient, userId, 'visibility')
    } catch (error) {
      const failure = finalRoundVisibilityFailure(
        error instanceof Error ? error : new Error('Ukjent feil'),
      )
      if (failure?.refetch) await visibilityQuery.refetch()
    }
  }

  if (visibilityQuery.isPending) {
    return (
      <div className="final-visibility-control loading" aria-busy="true">
        <LoaderCircle aria-hidden="true" />
        <div><h3>Finalens bakni</h3><p role="status">Henter synlighetsstatus …</p></div>
      </div>
    )
  }

  if (visibilityQuery.error && !visibilityQuery.data) {
    return (
      <div className="final-visibility-control error" role="alert">
        <div><h3>Finalens bakni</h3><p>{visibilityQuery.error.message}</p></div>
        <button type="button" onClick={() => void visibilityQuery.refetch()}>
          <RefreshCw aria-hidden="true" /> Prøv igjen
        </button>
      </div>
    )
  }

  const visibility = visibilityQuery.data
  if (!visibility) return null
  const released = !visibility.back_nine_hidden
  const failure = finalRoundVisibilityFailure(mutation.error)
  const attemptedState = mutation.variables

  return (
    <div className="final-visibility-control" aria-busy={mutation.isPending}>
      <div className="final-visibility-heading">
        {released ? <Eye aria-hidden="true" /> : <EyeOff aria-hidden="true" />}
        <div>
          <h3>Finalens bakni</h3>
          <p>{finalRound.name} · {released ? 'Hull 10–18 er frigitt' : 'Hull 10–18 er skjult'}</p>
        </div>
      </div>
      <label className="final-visibility-switch">
        <input
          type="checkbox"
          role="switch"
          checked={released}
          disabled={mutation.isPending || visibilityQuery.isFetching}
          aria-describedby="final-visibility-description final-visibility-sync"
          onChange={(event) => void save(!event.currentTarget.checked)}
        />
        <span>Frigi hull 10–18</span>
      </label>
      <p id="final-visibility-description" className="final-visibility-description">
        Administratoren styrer synligheten uten tidsfrist. Du kan skjule bakni igjen også etter at finalen er fullført eller låst.
      </p>
      <p id="final-visibility-sync" className="final-visibility-sync" role="status" aria-live="polite">
        {mutation.isPending
          ? attemptedState ? 'Skjuler hull 10–18 …' : 'Frigir hull 10–18 …'
          : receipt ?? 'Gjeldende status er synkronisert med serveren.'}
      </p>
      {failure && (
        <div className="final-visibility-error" role="alert">
          <p>{failure.message}</p>
          <button type="button" disabled={mutation.isPending} onClick={() => {
            if (failure.refetch) void visibilityQuery.refetch()
            else if (attemptedState !== undefined) void save(attemptedState)
          }}>
            <RefreshCw aria-hidden="true" /> {failure.refetch ? 'Hent status på nytt' : 'Prøv lagring igjen'}
          </button>
        </div>
      )}
      {receipt && !mutation.isPending && (
        <span className="final-visibility-confirmed"><CheckCircle2 aria-hidden="true" /> Bekreftet</span>
      )}
    </div>
  )
}
