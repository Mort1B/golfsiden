import { useAuth } from '../auth/authContext'
import { CheckCircle2, LockKeyhole, RefreshCw, Save } from 'lucide-react'
import { isTieBreakPolicy } from '../../api/tieBreakPolicy'
import { tieBreakExplanation, tieBreakLabel } from '../leaderboards/tieBreakExplanation'
import { useCountedRoundsEditor, type CountedRoundsEditorProps } from './useCountedRoundsEditor'

export function CountedRoundsEditor(props: CountedRoundsEditorProps) {
  const { session } = useAuth()
  return <CountedRoundsForm key={`${session?.user_id}:${session?.csrf_token}:${props.tournament.id}`} {...props} />
}

function CountedRoundsForm(props: CountedRoundsEditorProps) {
  const editor = useCountedRoundsEditor(props)
  const { value, mandatoryRoundId, policy, receipt, busy, editable, disabled, unchanged, failure, configurationIncoherent } = editor
  const tournamentLocked = props.tournament.status !== 'draft'
  const savedMandatoryLabel = props.tournament.mandatory_round_id === null ? 'ingen'
    : props.rounds?.find((round) => round.id === props.tournament.mandatory_round_id)?.name ?? 'valgt runde (navn utilgjengelig)'
  return (
    <form className="counted-rounds-editor" aria-busy={busy} onSubmit={(event) => { event.preventDefault(); void editor.save() }}>
      <div>
        <h3>Tellende runder og lik totalscore</h3>
        <p>Lagret valg: Beste {props.tournament.counted_rounds} av {props.tournament.number_of_rounds} runder. Obligatorisk: {savedMandatoryLabel}.</p>
        <p>Lik totalscore: {tieBreakLabel(props.tournament.tie_break_policy)}.</p>
        <p>En obligatorisk runde reserverer én av de tellende plassene, selv om den ikke er blant de beste resultatene.</p>
      </div>
      {!tournamentLocked && editor.refreshing && <p className="counted-rounds-state" role="status">Kontrollerer om valget kan endres …</p>}
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
      {(tournamentLocked || (!editor.refreshing && !props.roundsError && !editable && !configurationIncoherent)) && (
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
                disabled={disabled}
                onChange={(event) => {
                  editor.changeCount(Number(event.target.value))
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
                disabled={disabled}
                onChange={(event) => {
                  editor.changeMandatory(event.target.value || null)
                }}
              >
                <option value="">Ingen</option>
                {props.rounds?.map((round) => (
                  <option key={round.id} value={round.id}>Runde {round.round_number}: {round.name}</option>
                ))}
              </select>
            </label>
            <label className="counted-rounds-tie-policy" htmlFor="management-tie-break"><span>Ved lik totalscore sammenlagt</span>
              <select id="management-tie-break" value={policy} disabled={disabled} aria-describedby="management-tie-break-help"
                onChange={(event) => { if (isTieBreakPolicy(event.target.value)) editor.changePolicy(event.target.value) }}>
                <option value="shared_positions">Delt plass</option>
                <option value="final_round_score">Siste runde, deretter delt plass</option>
              </select>
            </label>
          </div>
          <p id="management-tie-break-help" className="counted-rounds-state counted-rounds-tie-policy">{tieBreakExplanation(policy)}</p>
          <p className="counted-rounds-state">Kan endres frem til turneringen startes.</p>
          <button className="counted-rounds-save" type="submit" disabled={disabled || unchanged}>
            <Save aria-hidden="true" /> {busy ? 'Lagrer …' : failure ? 'Prøv lagring igjen' : 'Lagre valg'}
          </button>
        </>
      )}
      {editor.refreshFailed && <div className="counted-rounds-message error" role="alert">
        <p>Kunne ikke oppdatere innstillingene. Kontroller lagret valg før du prøver igjen.</p>
        <button type="button" disabled={busy} onClick={() => void editor.refresh()}><RefreshCw aria-hidden="true" /> Oppdater innstillingene</button>
      </div>}
      {failure && <p className="counted-rounds-message error" role="alert">{failure.message}</p>}
      <p className="counted-rounds-receipt" aria-live="polite">{receipt && <><CheckCircle2 aria-hidden="true" /> {receipt}</>}</p>
    </form>
  )
}
