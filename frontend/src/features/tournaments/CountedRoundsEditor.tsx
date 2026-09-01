import { useState, type FormEvent } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { CheckCircle2, LockKeyhole, RefreshCw, Save } from 'lucide-react'
import { tournamentApi, tournamentKeys } from '../../api/tournaments'
import type { Round, Tournament } from '../../api/types'
import { useAuth } from '../auth/authContext'
import { countedRoundsAreEditable, countedRoundsFailure } from './countedRoundsEditor'
import { matchMandatoryRound, validateMandatoryRound } from '../../api/mandatoryRounds'

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
  const [draftMandatoryRoundId, setDraftMandatoryRoundId] = useState<string | null | undefined>(undefined)
  const [receipt, setReceipt] = useState<string | null>(null)
  const [localError, setLocalError] = useState<Error | null>(null)
  const value = draftValue ?? props.tournament.counted_rounds
  const mandatoryRoundId = draftMandatoryRoundId === undefined
    ? props.tournament.mandatory_round_id
    : draftMandatoryRoundId
  const mandatoryMatch = props.rounds === undefined
    ? null
    : matchMandatoryRound(props.tournament.mandatory_round_id, props.rounds)
  const configurationIncoherent = mandatoryMatch?.state === 'missing'
  const editable = countedRoundsAreEditable(props.tournament.status, props.rounds) && !configurationIncoherent
  const tournamentLocked = props.tournament.status !== 'draft'

  const mutation = useMutation({
    mutationFn: (configuration: { countedRounds: number; mandatoryRoundId: string | null }) => {
      const csrfToken = auth.session?.csrf_token
      if (!csrfToken) throw new Error('Økten mangler. Logg inn på nytt.')
      return tournamentApi.updateCountedRounds(props.tournament.id, {
        counted_rounds: configuration.countedRounds,
        mandatory_round_id: configuration.mandatoryRoundId,
        expected_tournament_updated_at: props.tournament.updated_at,
      }, csrfToken)
    },
  })

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!editable || mutation.isPending || unchanged) return
    mutation.reset()
    setLocalError(null)
    setReceipt(null)
    try {
      const saved = await mutation.mutateAsync({ countedRounds: value, mandatoryRoundId })
      const mandatory = validateMandatoryRound(
        saved.mandatory_round_id,
        props.rounds ?? [],
        'turneringsdata',
        'tournament.mandatory_round_id round identity',
      )
      queryClient.setQueryData(tournamentKeys.detail(userId, saved.id), saved)
      setDraftValue(null)
      setDraftMandatoryRoundId(undefined)
      const mandatoryLabel = saved.mandatory_round_id === null
        ? 'ingen'
        : mandatory?.name ?? 'valgt runde (navn utilgjengelig)'
      setReceipt(`Lagret: Beste ${saved.counted_rounds} av ${saved.number_of_rounds} runder. Obligatorisk: ${mandatoryLabel}.`)
      await queryClient.invalidateQueries({ queryKey: tournamentKeys.root(userId) })
    } catch (error) {
      const caught = error instanceof Error ? error : new Error('Ukjent feil')
      setLocalError(caught)
      const failure = countedRoundsFailure(caught)
      if (failure?.refetch) {
        setDraftValue(null)
        setDraftMandatoryRoundId(undefined)
        await queryClient.invalidateQueries({ queryKey: tournamentKeys.root(userId) })
      }
    }
  }

  const failure = countedRoundsFailure(localError ?? mutation.error)
  const unchanged = value === props.tournament.counted_rounds
    && mandatoryRoundId === props.tournament.mandatory_round_id
  const savedMandatoryRound = props.rounds?.find((round) => round.id === props.tournament.mandatory_round_id)
  const savedMandatoryLabel = props.tournament.mandatory_round_id === null
    ? 'ingen'
    : savedMandatoryRound?.name ?? 'valgt runde (navn utilgjengelig)'

  return (
    <form className="counted-rounds-editor" aria-busy={mutation.isPending} onSubmit={(event) => void submit(event)}>
      <div>
        <h3>Tellende runder</h3>
        <p>Lagret valg: Beste {props.tournament.counted_rounds} av {props.tournament.number_of_rounds} runder. Obligatorisk: {savedMandatoryLabel}.</p>
      </div>
      {!tournamentLocked && props.roundsPending && <p className="counted-rounds-state" role="status">Kontrollerer om valget kan endres …</p>}
      {!tournamentLocked && props.roundsError && (
        <div className="counted-rounds-message error" role="alert">
          <p>Kunne ikke kontrollere rundestatus. Valget kan ikke endres før oppdateringen lykkes.</p>
          <button type="button" onClick={props.onRetryRounds}><RefreshCw aria-hidden="true" /> Prøv igjen</button>
        </div>
      )}
      {configurationIncoherent && (
        <p className="counted-rounds-message error" role="alert">
          Lagret obligatorisk runde finnes ikke i turneringens rundeliste. Valget kan ikke brukes før turneringsdataene er oppdatert.
        </p>
      )}
      {(tournamentLocked || (!props.roundsPending && !props.roundsError && !editable && !configurationIncoherent)) && (
        <p className="counted-rounds-locked"><LockKeyhole aria-hidden="true" />
          {tournamentLocked
            ? 'Valget er låst fordi turneringen er startet, eller fordi en runde har vært åpnet og konfigurasjonen er fryst.'
            : 'Valget er permanent låst fordi minst én runde ikke lenger er et utkast.'}
        </p>
      )}
      {editable && (
        <>
          <div className="counted-rounds-fields">
            <label htmlFor="management-counted-rounds"><span>Antall som teller</span>
              <select
                id="management-counted-rounds"
                value={value}
                disabled={mutation.isPending}
                onChange={(event) => {
                  mutation.reset()
                  setLocalError(null)
                  setReceipt(null)
                  setDraftValue(Number(event.target.value))
                }}
              >
                {Array.from({ length: props.tournament.number_of_rounds }, (_, index) => (
                  <option key={index + 1} value={index + 1}>{index + 1}</option>
                ))}
              </select>
            </label>
            <label htmlFor="management-mandatory-round"><span>Obligatorisk runde</span>
              <select
                id="management-mandatory-round"
                value={mandatoryRoundId ?? ''}
                disabled={mutation.isPending}
                onChange={(event) => {
                  mutation.reset()
                  setLocalError(null)
                  setReceipt(null)
                  setDraftMandatoryRoundId(event.target.value || null)
                }}
              >
                <option value="">Ingen</option>
                {props.rounds?.map((round) => (
                  <option key={round.id} value={round.id}>Runde {round.round_number}: {round.name}</option>
                ))}
              </select>
            </label>
          </div>
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
