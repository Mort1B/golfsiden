import { useEffect, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { tournamentApi } from '../../api/tournaments'
import { matchMandatoryRound, validateMandatoryRound } from '../../api/mandatoryRounds'
import type { Round, Tournament, TournamentTieBreakPolicy } from '../../api/types'
import { useAuth } from '../auth/authContext'
import { tieBreakLabel } from '../leaderboards/tieBreakExplanation'
import { countedRoundsAreEditable, countedRoundsFailure } from './countedRoundsEditor'
import { reconcileTournamentClosure } from './reconcileTournamentClosure'

export interface CountedRoundsEditorProps {
  tournament: Tournament
  rounds: Round[] | undefined
  roundsPending: boolean
  roundsError: Error | null
  authorityRefreshing: boolean
  onRetryRounds: () => void
}

export function useCountedRoundsEditor(props: CountedRoundsEditorProps) {
  const auth = useAuth()
  const client = useQueryClient()
  const identity = `${auth.session?.user_id}:${auth.session?.csrf_token}:${props.tournament.id}`
  const currentIdentity = useRef(identity)
  currentIdentity.current = identity
  const alive = useRef(false)
  const submitting = useRef(false)
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  const [draftValue, setDraftValue] = useState<number | null>(null)
  const [draftMandatoryRoundId, setDraftMandatoryRoundId] = useState<string | null | undefined>(undefined)
  const [draftPolicy, setDraftPolicy] = useState<TournamentTieBreakPolicy | null>(null)
  const [receipt, setReceipt] = useState<string | null>(null)
  const [error, setError] = useState<Error | null>(null)
  const [busy, setBusy] = useState(false)
  const [refreshFailed, setRefreshFailed] = useState(false)
  const value = draftValue ?? props.tournament.counted_rounds
  const mandatoryRoundId = draftMandatoryRoundId === undefined ? props.tournament.mandatory_round_id : draftMandatoryRoundId
  const policy = draftPolicy ?? props.tournament.tie_break_policy
  const configurationIncoherent = props.rounds !== undefined
    && matchMandatoryRound(props.tournament.mandatory_round_id, props.rounds).state === 'missing'
  const editable = countedRoundsAreEditable(props.tournament.status, props.rounds) && !configurationIncoherent
  const refreshing = props.roundsPending || props.authorityRefreshing || auth.loading
  const disabled = busy || refreshing || !!props.roundsError || !!auth.error || !auth.session || refreshFailed
  const unchanged = value === props.tournament.counted_rounds
    && mandatoryRoundId === props.tournament.mandatory_round_id && policy === props.tournament.tie_break_policy
  const mutation = useMutation({
    mutationFn: () => {
      if (!auth.session) throw new Error('Økten mangler. Logg inn på nytt.')
      return tournamentApi.updateCountedRounds(props.tournament.id, {
        counted_rounds: value, mandatory_round_id: mandatoryRoundId, tie_break_policy: policy,
        expected_tournament_updated_at: props.tournament.updated_at,
      }, auth.session.csrf_token)
    }, retry: false,
  })
  const resetDraft = () => { setDraftValue(null); setDraftMandatoryRoundId(undefined); setDraftPolicy(null) }
  const clearMessage = () => { setError(null); setReceipt(null) }
  const run = async (save: boolean) => {
    if (submitting.current || (save && (!editable || disabled || unchanged))) return
    const startedIdentity = identity
    const isCurrent = () => alive.current && currentIdentity.current === startedIdentity
    submitting.current = true
    setBusy(true)
    let savedReceipt: string | null = null
    let saved = false
    let shouldReset = false
    if (save) {
      clearMessage()
      try {
        const result = await mutation.mutateAsync()
        const mandatory = validateMandatoryRound(result.mandatory_round_id, props.rounds ?? [], 'turneringsdata', 'tournament.mandatory_round_id round identity')
        savedReceipt = `Lagret: Beste ${result.counted_rounds} av ${result.number_of_rounds} runder. Obligatorisk: ${mandatory?.name ?? 'ingen'}. Lik totalscore: ${tieBreakLabel(result.tie_break_policy)}.`
        saved = true
        shouldReset = true
      } catch (caught) {
        const failure = caught instanceof Error ? caught : new Error('Ukjent feil')
        if (isCurrent()) setError(failure)
        shouldReset = countedRoundsFailure(failure)?.refetch === true
      }
    }
    try {
      if (!isCurrent()) return
      await reconcileTournamentClosure(client, auth.session?.user_id ?? '', props.tournament.id)
      if (isCurrent()) {
        setRefreshFailed(false)
        if (shouldReset) resetDraft()
        if (saved) setReceipt(savedReceipt)
      }
    } catch {
      if (isCurrent()) { setRefreshFailed(true); if (saved) resetDraft() }
    } finally {
      submitting.current = false
      if (isCurrent()) setBusy(false)
    }
  }
  return {
    value, mandatoryRoundId, policy, receipt, busy, refreshFailed, refreshing, editable, disabled,
    unchanged, configurationIncoherent, failure: countedRoundsFailure(error),
    save: () => run(true), refresh: () => run(false),
    changeCount: (next: number) => { clearMessage(); setDraftValue(next) },
    changeMandatory: (next: string | null) => { clearMessage(); setDraftMandatoryRoundId(next) },
    changePolicy: (next: TournamentTieBreakPolicy) => { clearMessage(); setDraftPolicy(next) },
  }
}
