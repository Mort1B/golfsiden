import { useState, type FormEvent } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { CheckCircle2, Pencil, X } from 'lucide-react'
import { api } from '../../api/client'
import { ApiHttpError } from '../../api/http'
import { tournamentKeys } from '../../api/tournaments'
import type { TournamentPlayer, TournamentPlayerRoster } from '../../api/types'
import { EmptyState, ErrorState, LoadingState } from '../../ui/AsyncState'
import { useAuth } from '../auth/authContext'
import { formatHandicap, parseHandicap } from '../handicap/format'

interface TournamentPlayerSectionProps {
  tournamentId: string
  isAdmin: boolean
  roster: TournamentPlayerRoster | undefined
  pending: boolean
  error: Error | null
  onRetry: () => void
  adminAccessPending: boolean
  adminAccessError: Error | null
}

function mutationMessage(error: Error | null): string | null {
  if (error instanceof ApiHttpError) {
    if (error.code === 'tournament_handicap_locked') {
      return 'Handicapet er låst fordi en runde har vært åpnet.'
    }
    if (error.code === 'tournament_handicap_unchanged') {
      return 'Skriv inn et handicap som er forskjellig fra det lagrede.'
    }
  }
  return error?.message ?? null
}

export function TournamentPlayerSection(props: TournamentPlayerSectionProps) {
  const auth = useAuth()
  const queryClient = useQueryClient()
  const userId = auth.session?.user_id ?? ''
  const [editingId, setEditingId] = useState<string | null>(null)
  const [handicap, setHandicap] = useState('')
  const [reason, setReason] = useState('')
  const [validationError, setValidationError] = useState<string | null>(null)
  const [receipt, setReceipt] = useState<string | null>(null)

  const correction = useMutation({
    mutationFn: ({ playerId, handicapIndex, correctionReason }: {
      playerId: string
      handicapIndex: number
      correctionReason: string
    }) => {
      const csrfToken = auth.session?.csrf_token
      if (!csrfToken) throw new Error('Økten mangler. Logg inn på nytt.')
      return api.correctTournamentHandicap(
        props.tournamentId,
        playerId,
        { handicap_index: handicapIndex, reason: correctionReason },
        csrfToken,
      )
    },
  })

  const beginCorrection = (player: TournamentPlayer) => {
    correction.reset()
    setReceipt(null)
    setValidationError(null)
    setHandicap(formatHandicap(player.tournament_handicap))
    setReason('')
    setEditingId(player.player_id)
  }

  const cancelCorrection = () => {
    correction.reset()
    setValidationError(null)
    setEditingId(null)
  }

  const submit = async (event: FormEvent<HTMLFormElement>, player: TournamentPlayer) => {
    event.preventDefault()
    const parsed = parseHandicap(handicap)
    if (!parsed.ok) {
      setValidationError(parsed.message)
      return
    }
    if (!reason.trim()) {
      setValidationError('Skriv hvorfor handicapet korrigeres.')
      return
    }
    setValidationError(null)
    setReceipt(null)
    try {
      const result = await correction.mutateAsync({
        playerId: player.player_id,
        handicapIndex: parsed.value,
        correctionReason: reason.trim(),
      })
      queryClient.setQueryData<TournamentPlayerRoster>(tournamentKeys.players(userId, props.tournamentId), (current) => current ? {
        ...current,
        players: current.players.map((entry) => entry.player_id === result.player.player_id ? result.player : entry),
      } : current)
      await queryClient.invalidateQueries({ queryKey: tournamentKeys.players(userId, props.tournamentId) })
      setReceipt(`${result.player.display_name} er oppdatert til HCP ${formatHandicap(result.player.tournament_handicap)}. Endringen er lagret i historikken.`)
      setEditingId(null)
    } catch (error) {
      if (error instanceof ApiHttpError && error.code === 'tournament_handicap_locked') {
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: tournamentKeys.players(userId, props.tournamentId) }),
          queryClient.invalidateQueries({ queryKey: tournamentKeys.rounds(userId, props.tournamentId) }),
        ])
      }
    }
  }

  const correctionState = props.roster?.handicap_correction
  return (
    <section className="tournament-players" aria-labelledby="tournament-players-heading">
      <div className="section-heading">
        <h2 id="tournament-players-heading">Spillere</h2>
        <span>{props.roster?.players.length ?? 0}</span>
      </div>
      {props.pending && <LoadingState />}
      {props.error && <ErrorState error={props.error} onRetry={props.onRetry} />}
      {props.adminAccessPending && <p className="handicap-access-notice" role="status">Kontrollerer administratortilgang …</p>}
      {props.adminAccessError && <p className="handicap-access-notice error" role="alert">Kunne ikke kontrollere administratortilgangen. Spillerlisten vises uten redigering.</p>}
      {!props.isAdmin && !props.adminAccessPending && !props.adminAccessError && props.roster && (
        <p className="handicap-access-notice">Turneringshandicap kan bare korrigeres av en turneringsadministrator før første runde åpnes.</p>
      )}
      {correctionState?.state === 'locked' && props.isAdmin && (
        <p className="handicap-lock-notice" role="status">
          Turneringshandicap er permanent låst fordi {correctionState.reason === 'round_opened' ? 'en runde har vært åpnet' : 'et rundesnapshot er lagret'}. Endringer i spillerprofilen gjelder bare fremtidige turneringer.
        </p>
      )}
      {props.roster?.players.length === 0 && <EmptyState>Ingen spillere er påmeldt.</EmptyState>}
      {props.roster && props.roster.players.length > 0 && (
        <div className="tournament-player-list">
          {props.roster.players.map((player) => (
            <article className="tournament-player-row" key={player.player_id}>
              <div className="tournament-player-summary">
                <div><strong>{player.display_name}</strong>{player.status === 'withdrawn' && <span>Trukket</span>}</div>
                <span>HCP {formatHandicap(player.tournament_handicap)}</span>
                {props.isAdmin && correctionState?.state === 'editable' && editingId !== player.player_id && (
                  <button type="button" onClick={() => beginCorrection(player)} disabled={correction.isPending}>
                    <Pencil aria-hidden="true" /> Korriger
                  </button>
                )}
              </div>
              {editingId === player.player_id && correctionState?.state === 'editable' && (
                <form className="handicap-correction-form" aria-busy={correction.isPending} onSubmit={(event) => void submit(event, player)}>
                  <label><span>Nytt handicap</span><input type="text" inputMode="decimal" required value={handicap} onChange={(event) => setHandicap(event.target.value)} disabled={correction.isPending} /></label>
                  <label><span>Årsak til korrigering</span><textarea required maxLength={500} rows={3} value={reason} onChange={(event) => setReason(event.target.value)} disabled={correction.isPending} /></label>
                  {(validationError || correction.error) && <p className="handicap-correction-error" role="alert">{validationError ?? mutationMessage(correction.error)}</p>}
                  <div className="handicap-correction-actions">
                    <button type="button" className="secondary" onClick={cancelCorrection} disabled={correction.isPending}><X aria-hidden="true" /> Avbryt</button>
                    <button type="submit" disabled={correction.isPending}><CheckCircle2 aria-hidden="true" /> {correction.isPending ? 'Lagrer …' : 'Lagre korrigering'}</button>
                  </div>
                </form>
              )}
            </article>
          ))}
        </div>
      )}
      <p className="handicap-correction-receipt" aria-live="polite">{receipt}</p>
    </section>
  )
}
