import { useState, type FormEvent } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { CheckCircle2, LockKeyhole, RefreshCw, Save } from 'lucide-react'
import { tournamentApi, tournamentKeys } from '../../api/tournaments'
import type { Round, Tournament } from '../../api/types'
import { useAuth } from '../auth/authContext'
import { countedRoundsAreEditable, countedRoundsFailure } from './countedRoundsEditor'

interface CountedRoundsEditorProps {
  tournament: Tournament
  rounds: Round[] | undefined
  roundsPending: boolean
  roundsError: Error | null
  onRetryRounds: () => void
}

export function CountedRoundsEditor(props: CountedRoundsEditorProps) {
  const auth = useAuth()
  const queryClient = useQueryClient()
  const userId = auth.session?.user_id ?? ''
  const [draftValue, setDraftValue] = useState<number | null>(null)
  const [receipt, setReceipt] = useState<string | null>(null)
  const value = draftValue ?? props.tournament.counted_rounds
  const editable = countedRoundsAreEditable(props.tournament.status, props.rounds)
  const tournamentLocked = props.tournament.status !== 'draft'

  const mutation = useMutation({
    mutationFn: (countedRounds: number) => {
      const csrfToken = auth.session?.csrf_token
      if (!csrfToken) throw new Error('Økten mangler. Logg inn på nytt.')
      return tournamentApi.updateCountedRounds(props.tournament.id, {
        counted_rounds: countedRounds,
        expected_tournament_updated_at: props.tournament.updated_at,
      }, csrfToken)
    },
  })

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!editable || mutation.isPending || value === props.tournament.counted_rounds) return
    mutation.reset()
    setReceipt(null)
    try {
      const saved = await mutation.mutateAsync(value)
      queryClient.setQueryData(tournamentKeys.detail(userId, saved.id), saved)
      setDraftValue(null)
      setReceipt(`Lagret: Beste ${saved.counted_rounds} av ${saved.number_of_rounds} runder.`)
      await queryClient.invalidateQueries({ queryKey: tournamentKeys.root(userId) })
    } catch (error) {
      const failure = countedRoundsFailure(error instanceof Error ? error : new Error('Ukjent feil'))
      if (failure?.refetch) {
        setDraftValue(null)
        await queryClient.invalidateQueries({ queryKey: tournamentKeys.root(userId) })
      }
    }
  }

  const failure = countedRoundsFailure(mutation.error)
  const unchanged = value === props.tournament.counted_rounds

  return (
    <form className="counted-rounds-editor" aria-busy={mutation.isPending} onSubmit={(event) => void submit(event)}>
      <div>
        <h3>Tellende runder</h3>
        <p>Lagret valg: Beste {props.tournament.counted_rounds} av {props.tournament.number_of_rounds} runder.</p>
      </div>
      {!tournamentLocked && props.roundsPending && <p className="counted-rounds-state" role="status">Kontrollerer om valget kan endres …</p>}
      {!tournamentLocked && props.roundsError && (
        <div className="counted-rounds-message error" role="alert">
          <p>Kunne ikke kontrollere rundestatus. Valget kan ikke endres før oppdateringen lykkes.</p>
          <button type="button" onClick={props.onRetryRounds}><RefreshCw aria-hidden="true" /> Prøv igjen</button>
        </div>
      )}
      {(tournamentLocked || (!props.roundsPending && !props.roundsError && !editable)) && (
        <p className="counted-rounds-locked"><LockKeyhole aria-hidden="true" />
          {tournamentLocked
            ? 'Valget er låst fordi turneringen ikke lenger er i kladd.'
            : 'Valget er permanent låst fordi minst én runde ikke lenger er et utkast.'}
        </p>
      )}
      {editable && (
        <>
          <label htmlFor="management-counted-rounds"><span>Antall som teller</span>
            <select
              id="management-counted-rounds"
              value={value}
              disabled={mutation.isPending}
              onChange={(event) => {
                mutation.reset()
                setReceipt(null)
                setDraftValue(Number(event.target.value))
              }}
            >
              {Array.from({ length: props.tournament.number_of_rounds }, (_, index) => (
                <option key={index + 1} value={index + 1}>{index + 1}</option>
              ))}
            </select>
          </label>
          <p className="counted-rounds-state">Kan endres frem til turneringen startes.</p>
          <button className="counted-rounds-save" type="submit" disabled={mutation.isPending || unchanged}>
            <Save aria-hidden="true" /> {mutation.isPending ? 'Lagrer …' : failure ? 'Prøv lagring igjen' : 'Lagre valg'}
          </button>
        </>
      )}
      {failure && <p className="counted-rounds-message error" role="alert">{failure.message}</p>}
      <p className="counted-rounds-receipt" aria-live="polite">{receipt && <><CheckCircle2 aria-hidden="true" /> {receipt}</>}</p>
    </form>
  )
}
