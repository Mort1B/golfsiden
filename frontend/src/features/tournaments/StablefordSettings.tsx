import { useLayoutEffect, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { stablefordApi } from '../../api/stableford'
import { ApiHttpError } from '../../api/http'
import { tournamentKeys } from '../../api/tournaments'
import type { Round } from '../../api/types'
import { useAuth } from '../auth/authContext'
import { authKeys, type AuthSession } from '../../api/auth'
import './stablefordSettings.css'
export function StablefordSettings({ round }: { round: Round }) {
  const { session } = useAuth()
  return <section className="stableford-settings" aria-label={`Stableford-innstillinger for ${round.name}`}>
    <h3>{round.name} · Stableford</h3>
    <p>18 hull · individuelt · felles utslagssted</p>
    <p>Sammenlagt: 36 minus poeng for fullt kort, ellers 2 per løste hull minus poeng. Pickup gir +2; tomme hull bidrar ikke. Opprinnelige slag bevares. Slagspill bruker fortsatt ubegrenset score mot par.</p>
    {round.status === 'draft' ? <SettingsForm key={`${session?.user_id}:${session?.csrf_token}:${round.tournament_id}:${round.id}`} round={round} /> : <p>
      Innstillinger låst ved åpning: {round.handicap_enabled ? `${round.handicap_allowance_percent} % handicap` : 'Handicap av (0 mottatte slag)'}. Scorekortet bruker bevart turneringshandicap og utslagssted.
    </p>}
  </section>
}
function SettingsForm({ round }: { round: Round }) {
  const { session } = useAuth()
  const client = useQueryClient()
  const [draft, setDraft] = useState<{ enabled: boolean; allowance: string } | null>(null)
  const enabled = draft?.enabled ?? round.handicap_enabled
  const allowance = draft?.allowance ?? String(round.handicap_allowance_percent)
  const valid = /^\d{1,3}$/.test(allowance) && Number(allowance) <= 100
  const busy = useRef(false)
  const userId = session?.user_id ?? ''
  const alive = useRef(false)
  useLayoutEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  // The form's key fixes account, CSRF and target ownership for its lifetime.
  // Check canonical identity too: publication can precede React's next commit.
  const isCurrent = () => {
    const current = client.getQueryData<AuthSession | null>(authKeys.session)
    return alive.current && !!session && current?.user_id === session.user_id && current.csrf_token === session.csrf_token
  }
  const refresh = () => Promise.all([
    tournamentKeys.rounds(userId, round.tournament_id),
    tournamentKeys.round(userId, round.id),
  ].map(queryKey => isCurrent() ? client.invalidateQueries({ queryKey, exact: true }) : Promise.resolve()))
  const mutation = useMutation({ retry: false, gcTime: 0,
    mutationFn: async () => {
      if (!session || !isCurrent() || !valid || round.status !== 'draft') throw new Error('Kontroller innstillingene før lagring.')
      return stablefordApi.settings(round, enabled, Number(allowance), session.csrf_token)
    },
    onSuccess: async updated => {
      if (!isCurrent()) return
      client.setQueryData(tournamentKeys.round(userId, round.id), updated)
      if (!isCurrent()) return
      client.setQueryData<Round[]>(tournamentKeys.rounds(userId, round.tournament_id), current => current?.map(item => item.id === updated.id ? updated : item))
      if (!isCurrent()) return
      setDraft(null)
      await refresh()
    },
    onError: async error => { if (isCurrent() && error instanceof ApiHttpError && ['round_not_draft', 'round_configuration_stale'].includes(error.code ?? '')) { await refresh(); if (isCurrent()) setDraft(null) } },
    onSettled: () => { if (isCurrent()) busy.current = false },
  })
  const error = mutation.error instanceof ApiHttpError && mutation.error.code === 'round_configuration_stale'
    ? 'Runden ble endret av andre. Utkastet er erstattet med gjeldende innstillinger. Kontroller dem før du prøver igjen.'
    : mutation.error instanceof ApiHttpError && mutation.error.code === 'round_not_draft' ? 'Runden er åpnet. Innstillingene kan ikke endres.'
      : mutation.error ? 'Innstillingene kunne ikke lagres. Prøv igjen.' : null
  return <form onSubmit={event => { event.preventDefault(); if (!busy.current && valid) { busy.current = true; mutation.mutate() } }}>
    <label><input type="checkbox" checked={enabled} disabled={mutation.isPending} onChange={event => { mutation.reset(); setDraft({ allowance, enabled: event.target.checked }) }} />Bruk handicap</label>
    <label htmlFor={`allowance-${round.id}`}>Handicapandel (0–100 %)</label>
    <input id={`allowance-${round.id}`} inputMode="numeric" type="number" min="0" max="100" step="1" value={allowance} disabled={mutation.isPending}
      aria-invalid={!valid} aria-describedby={!valid ? `allowance-error-${round.id}` : undefined}
      onChange={event => { mutation.reset(); setDraft({ enabled, allowance: event.target.value }) }} />
    {!valid && <p id={`allowance-error-${round.id}`} role="alert">Skriv et heltall fra 0 til 100.</p>}
    <p>Andelen brukes før én avrunding av banehandicap. Pluss-handicap og mottatte slag bevares ved åpning.</p>
    <button type="submit" disabled={!session || !valid || mutation.isPending}>{mutation.isPending ? 'Lagrer …' : 'Lagre Stableford-innstillinger'}</button>
    {mutation.isSuccess && <p role="status">Stableford-innstillingene er lagret.</p>}
    {error && <p role="alert">{error}</p>}
  </form>
}
