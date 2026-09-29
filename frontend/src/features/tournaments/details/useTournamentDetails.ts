import { useLayoutEffect, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from '../../../api/auth'
import { ApiHttpError } from '../../../api/http'
import { tournamentApi, tournamentKeys } from '../../../api/tournaments'
import type { Round, Tournament } from '../../../api/types'
import { useAuth } from '../../auth/authContext'
import { reconcileTournamentClosure } from '../reconcileTournamentClosure'
import { detailsErrors, detailsFields, type DetailsFields } from './detailsValidation'
export interface DetailsProps {
  tournament: Tournament
  rounds: Round[] | undefined
  loading: boolean
  readError: Error | null
  retry: () => void
}
export function useTournamentDetails(props: DetailsProps) {
  const { session } = useAuth(), client = useQueryClient()
  const alive = useRef(false), submitting = useRef(false)
  useLayoutEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  const isCurrent = () => {
    const current = client.getQueryData<AuthSession | null>(authKeys.session)
    return alive.current && !!session && current?.user_id === session.user_id && current.csrf_token === session.csrf_token
  }
  const [draft, setDraft] = useState<{ fields: DetailsFields; version: string } | null>(null)
  const [busy, setBusy] = useState(false), [receipt, setReceipt] = useState(false)
  const [failure, setFailure] = useState<string | null>(null), [mustRefresh, setMustRefresh] = useState(false)
  const fields = draft?.fields ?? detailsFields(props.tournament)
  const errors = detailsErrors(fields, props.rounds ?? [])
  const conflict = mustRefresh || !!draft && draft.version !== props.tournament.updated_at
  const unchanged = fields.name.trim() === props.tournament.name && fields.description.trim() === props.tournament.description
    && fields.start_date === props.tournament.start_date && fields.end_date === props.tournament.end_date
  const disabled = busy || props.loading || !!props.readError || !props.rounds || !session || props.tournament.status !== 'draft'
  const mutation = useMutation({ retry: false, gcTime: 0,
    mutationFn: () => {
      if (!session?.csrf_token || !isCurrent()) throw new Error('Økten mangler. Logg inn på nytt.')
      return tournamentApi.updateDetails(props.tournament.id, { ...fields,
        expected_tournament_updated_at: draft?.version ?? props.tournament.updated_at }, session.csrf_token)
    },
  })
  const refresh = async () => {
    if (!session || !isCurrent()) return false
    await reconcileTournamentClosure(client, session.user_id, props.tournament.id)
    if (!isCurrent()) return false
    await client.fetchQuery({ queryKey: tournamentKeys.detail(session.user_id, props.tournament.id),
      queryFn: () => tournamentApi.detail(props.tournament.id), staleTime: 0 })
    return isCurrent()
  }
  const run = async (save: boolean) => {
    if (!isCurrent() || submitting.current || (save && (disabled || unchanged || conflict || Object.keys(errors).length > 0))) return
    submitting.current = true; setBusy(true); setReceipt(false); setFailure(null)
    try {
      if (save) {
        await mutation.mutateAsync()
        if (!isCurrent()) return
        // A server-accepted write must be reconciled before another submission.
        setMustRefresh(true)
      }
      if (await refresh()) { setDraft(null); setMustRefresh(false); setReceipt(save) }
    } catch (error) {
      if (!isCurrent()) return
      const code = error instanceof ApiHttpError ? error.code : null
      if (code === 'tournament_details_stale' || code === 'tournament_details_locked') {
        setMustRefresh(true)
        setFailure(code === 'tournament_details_locked' ? 'Turneringen er startet. Opplysningene kan ikke endres.' : 'Turneringen ble endret et annet sted. Utkastet ditt er beholdt.')
        try { await refresh() } catch { if (isCurrent()) setFailure('Kunne ikke hente oppdatert turnering. Utkastet er beholdt; prøv å hente siste versjon igjen.') }
      } else if (code === 'tournament_details_round_dates') {
        setFailure('Datoene må inneholde alle rundene. Hent siste versjon for å kontrollere rundeplanen.'); setMustRefresh(true)
      } else setFailure('Kunne ikke lagre eller kontrollere turneringen. Utkastet er beholdt. Prøv igjen.')
    } finally {
      if (isCurrent()) { submitting.current = false; setBusy(false) }
    }
  }
  return { fields, errors, disabled, busy, receipt, failure, conflict, unchanged,
    save: () => run(true), reload: () => run(false),
    change: (key: keyof DetailsFields, value: string) => {
      if (!isCurrent() || busy) return
      setReceipt(false); setFailure(null)
      setDraft(current => ({ fields: { ...(current?.fields ?? detailsFields(props.tournament)), [key]: value }, version: current?.version ?? props.tournament.updated_at }))
    },
  }
}
