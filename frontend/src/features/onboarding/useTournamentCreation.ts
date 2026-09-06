import { useEffect, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router-dom'
import { creationApi, type CreateTournamentRequest, type TournamentPlanRequest } from '../../api/tournamentCreation'
import { ApiHttpError } from '../../api/http'
import { tournamentKeys } from '../../api/tournaments'
import type { AuthSession } from '../../api/auth'

export function useTournamentCreation(session: AuthSession | undefined) {
  const client = useQueryClient()
  const navigate = useNavigate()
  const alive = useRef(false)
  const busy = useRef(false)
  const ambiguous = useRef(false)
  const [saving, setSaving] = useState(false)
  const attempt = useRef<CreateTournamentRequest | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [uncertain, setUncertain] = useState(false)
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  const mutation = useMutation({ mutationFn: (input: CreateTournamentRequest) => {
    if (!session) throw new Error('Logg inn før du oppretter en turnering.')
    return creationApi.create(input, session.csrf_token)
  } })
  const submit = async (plan: TournamentPlanRequest) => {
    if (!session || busy.current) return false
    busy.current = true
    setSaving(true)
    setError(null)
    attempt.current ??= { ...plan, request_id: crypto.randomUUID() }
    try {
      const result = await mutation.mutateAsync(attempt.current)
      if (!alive.current) return false
      await Promise.all([
        client.invalidateQueries({ queryKey: tournamentKeys.mine(session.user_id), exact: true }),
        client.invalidateQueries({ queryKey: tournamentKeys.list(session.user_id), exact: true }),
      ])
      if (!alive.current) return false
      navigate(`/manage/tournaments/${result.tournament_id}#courses`)
      return true
    } catch (e) {
      if (!alive.current) return false
      const known = e instanceof ApiHttpError && [400,401,403,409,413,429].includes(e.status)
      ambiguous.current ||= !known || (e instanceof ApiHttpError && e.status === 409)
      setUncertain(ambiguous.current)
      if (known && !ambiguous.current) attempt.current = null
      setError(e instanceof ApiHttpError && e.status === 401 ? 'Økten er utløpt. Logg inn igjen før du fortsetter.'
        : e instanceof ApiHttpError && e.status === 429 ? 'For mange opprettingsforsøk. Vent litt og prøv igjen.'
        : e instanceof ApiHttpError && e.status === 409 ? 'Opprettingsforsøket samsvarer ikke med lagret forespørsel. Kontroller turneringene dine før du starter på nytt.'
        : known ? 'Opprettingen ble avvist. Kontroller opplysningene og tilgangen din før du prøver igjen.'
        : 'Vi fikk ikke bekreftet opprettingen. Prøv igjen med samme opplysninger; samme forsøk oppretter ikke en ekstra turnering. Du kan også kontrollere turneringene dine.')
      return false
    } finally { busy.current = false; if (alive.current) setSaving(false) }
  }
  return { submit, error, uncertain, submitting: saving }
}
