import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { authKeys, type AuthSession } from '../../../api/auth'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ApiHttpError } from '../../../api/http'
import { pairingApi, pairingKeys } from '../../../api/pairings'
import type { PairingReplacement, RoundPairings } from '../../../api/pairings'
import { tournamentKeys } from '../../../api/tournaments'
import type { Round } from '../../../api/types'
import { useAuth } from '../../auth/authContext'
import { draftFromPairings, pairingsFingerprint } from './draft'
import type { PairingDraft } from './draft'

interface Input { tournamentId: string; round: Round; expanded: boolean }

export type PairingFailure =
  | 'stale' | 'not-draft' | 'access' | 'identity' | 'roster' | 'legacy'
  | 'schedule' | 'referenced-team' | 'retryable' | 'session'

const failureByCode: Readonly<Record<string, PairingFailure>> = {
  round_pairings_stale: 'stale',
  round_not_draft: 'not-draft',
  pairing_identity_conflict: 'identity',
  invalid_pairing_roster: 'roster',
  legacy_mapping_required: 'legacy',
  invalid_legacy_conversion: 'legacy',
  invalid_schedule_transfer: 'schedule',
  team_is_referenced: 'referenced-team',
}

export function pairingFailure(error: unknown): PairingFailure | null {
  if (!error) return null
  if (error instanceof ApiHttpError) {
    if (error.status === 401 || error.status === 403 || error.status === 404) return 'access'
    return failureByCode[error.code] ?? 'retryable'
  }
  return error instanceof Error && error.message.includes('Økten mangler') ? 'session' : 'retryable'
}

export function pairingFailureMessage(failure: PairingFailure | null): string | null {
  switch (failure) {
    case 'stale': return 'Oppsettet er endret et annet sted. Forkast utkastet og last inn den nyeste versjonen.'
    case 'not-draft': return 'Runden er ikke lenger et utkast. Oppsettet kan bare leses.'
    case 'access': return 'Tilgangen til oppsettet ble avvist. Oppdater siden eller logg inn på nytt.'
    case 'identity': return 'En lag- eller flightidentitet kolliderer med lagret oppsett. Forkast utkastet og last inn på nytt.'
    case 'roster': return 'Deltakerlisten er endret. Forkast utkastet og last inn aktive deltakere på nytt.'
    case 'legacy': return 'De eldre individuelle gruppene må konverteres nøyaktig før videre redigering.'
    case 'schedule': return 'Et gammelt lag med starttid må overføres eksplisitt til en flight med nøyaktig de samme spillerne.'
    case 'referenced-team': return 'Laget har lagrede resultater og kan ikke fjernes.'
    case 'session': return 'Økten mangler. Logg inn på nytt før du lagrer.'
    case 'retryable': return 'Oppsettet kunne ikke lagres. Kontroller forbindelsen og prøv igjen.'
    default: return null
  }
}

export function usePairingEditor({ tournamentId, round, expanded }: Input) {
  const auth = useAuth()
  const userId = auth.session?.user_id ?? ''
  const queryClient = useQueryClient()
  const csrfToken = auth.session?.csrf_token
  const alive = useRef(false)
  useLayoutEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  // PairingEditor keys the owner by account, CSRF and target. Canonical
  // publication may precede React replacement, so mounted state is not enough.
  const isCurrent = useCallback(() => {
    const current = queryClient.getQueryData<AuthSession | null>(authKeys.session)
    return alive.current && !!csrfToken && current?.user_id === userId && current.csrf_token === csrfToken
  }, [queryClient, userId, csrfToken])
  const submitting = useRef(false)
  const [draft, setDraft] = useState<PairingDraft | null>(null)
  const [dirty, setDirty] = useState(false)
  const [reloadConflict, setReloadConflict] = useState(false)
  const query = useQuery({
    queryKey: pairingKeys.detail(userId, round.id),
    queryFn: () => pairingApi.get(round.id, tournamentId),
    enabled: expanded && userId.length > 0,
  })
  const mutation = useMutation({ retry: false, gcTime: 0, mutationFn: (replacement: PairingReplacement) => {
    if (!csrfToken || !isCurrent()) throw new Error('Økten mangler. Logg inn på nytt.')
    return pairingApi.replace(round.id, tournamentId, replacement, csrfToken)
  } })

  useEffect(() => {
    const authoritative = query.data
    if (!isCurrent() || !authoritative || draft?.sourceFingerprint === pairingsFingerprint(authoritative)) return
    if (dirty) {
      setReloadConflict(true)
      return
    }
    setDraft(draftFromPairings(authoritative))
    setReloadConflict(false)
  }, [dirty, draft?.sourceFingerprint, query.data, isCurrent])

  const adopt = (authoritative: RoundPairings) => {
    if (!isCurrent()) return
    setDraft(draftFromPairings(authoritative))
    setDirty(false)
    setReloadConflict(false)
  }

  const edit = (update: (current: PairingDraft) => PairingDraft) => {
    if (!isCurrent()) return
    setDraft((current) => current ? update(current) : current)
    setDirty(true)
    mutation.reset()
  }

  const invalidateRoundFacts = () => Promise.all([
    pairingKeys.detail(userId, round.id),
    tournamentKeys.round(userId, round.id),
    tournamentKeys.rounds(userId, tournamentId),
  ].map(queryKey => isCurrent()
    ? queryClient.invalidateQueries({ queryKey, exact: true }) : Promise.resolve()))

  const save = async (replacement: PairingReplacement): Promise<RoundPairings | null> => {
    if (!isCurrent() || submitting.current) return null
    submitting.current = true
    mutation.reset()
    try {
      const saved = await mutation.mutateAsync(replacement)
      if (!isCurrent()) return null
      queryClient.setQueryData(pairingKeys.detail(userId, round.id), saved)
      adopt(saved)
      await invalidateRoundFacts()
      return isCurrent() ? saved : null
    } catch (error) {
      if (!isCurrent()) return null
      const failure = pairingFailure(error)
      if (failure === 'stale' || failure === 'not-draft' || failure === 'access' || failure === 'roster') {
        setReloadConflict(true)
        await invalidateRoundFacts()
      }
      return null
    } finally {
      if (isCurrent()) submitting.current = false
    }
  }

  const discardAndReload = async () => {
    if (!isCurrent()) return
    mutation.reset()
    setDirty(false)
    setReloadConflict(false)
    if (query.data) adopt(query.data)
    const result = await query.refetch()
    if (isCurrent() && result.data) adopt(result.data)
  }

  return {
    query, mutation, draft, dirty, reloadConflict, edit, save, discardAndReload,
    failure: pairingFailure(mutation.error),
    refetch: () => { if (isCurrent()) return query.refetch() },
  }
}
