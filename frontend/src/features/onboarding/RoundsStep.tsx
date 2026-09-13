import { Plus, Trash2 } from 'lucide-react'
import type { FieldErrors } from './validation'
import type { RoundDraft, TournamentDraft } from './wizardState'
import { FieldError, WizardControls } from './WizardControls'
import type { RefObject } from 'react'
import { isScoringFormat } from '../../api/scoringFormats'
import { MANDATORY_ROUND_EXPLANATION } from '../leaderboards/resultExplanations'

interface RoundsStepProps {
  totalSteps?: number
  tournament: TournamentDraft
  rounds: RoundDraft[]
  countedRounds: number | null
  mandatoryRoundKey: string | null
  errors: FieldErrors
  onAdd: () => void
  onRemove: (key: string) => void
  onChange: (key: string, value: Partial<Omit<RoundDraft, 'key'>>) => void
  onCountedRounds: (value: number) => void
  onMandatoryRound: (key: string | null) => void
  onBack: () => void
  onNext: () => void
  headingRef: RefObject<HTMLHeadingElement | null>
}

export function RoundsStep(props: RoundsStepProps) {
  return (
    <section className="wizard-step" aria-labelledby="rounds-step-heading">
      <header><p className="eyebrow">Steg 2 av {props.totalSteps ?? 4}</p><h1 id="rounds-step-heading" ref={props.headingRef} tabIndex={-1}>Planlegg rundene</h1><p>Velg spilleform for hver runde. Bane og utslagssted kan settes senere.</p></header>
      <div className="counted-rounds-choice">
        <label htmlFor="counted-rounds">
          <span>Tellende runder</span>
          <select
            id="counted-rounds"
            value={props.countedRounds ?? ''} disabled={props.countedRounds === null}
            aria-invalid={Boolean(props.errors['rounds.countedRounds'])}
            aria-describedby="counted-rounds-help counted-rounds-error"
            onChange={(event) => props.onCountedRounds(Number(event.target.value))}
          >
            {props.countedRounds === null && <option value="">Egen matchpoengtabell</option>}
            {props.rounds.filter(r => r.scoringFormat !== 'singles_match_play').map((_, index) => <option key={index + 1} value={index + 1}>{index + 1}</option>)}
          </select>
        </label>
        <p id="counted-rounds-help">{props.countedRounds === null ? 'Matchspill har en egen poengtabell. Sammenlagt brutto/netto er ikke tilgjengelig.' : `Beste ${props.countedRounds} av ${props.rounds.filter(r => r.scoringFormat !== 'singles_match_play').length} tellende formater. Matchspill inngår ikke.`}</p>
        <FieldError id="counted-rounds-error">{props.errors['rounds.countedRounds']}</FieldError>
      </div>
      <div className="counted-rounds-choice mandatory-round-choice">
        <label htmlFor="mandatory-round">
          <span>Obligatorisk runde (valgfritt)</span>
          <select
            id="mandatory-round"
            disabled={props.countedRounds === null}
            value={props.mandatoryRoundKey ?? ''}
            aria-describedby="mandatory-round-help"
            onChange={(event) => props.onMandatoryRound(event.target.value || null)}
          >
            <option value="">Ingen obligatorisk runde</option>
            {props.rounds.filter(r => r.scoringFormat !== 'singles_match_play').map((round) => (
              <option key={round.key} value={round.key}>Runde {props.rounds.indexOf(round) + 1}: {round.name || 'Uten navn'}</option>
            ))}
          </select>
        </label>
        <p id="mandatory-round-help">{MANDATORY_ROUND_EXPLANATION}</p>
      </div>
      <div className="round-editor-list">
        {props.rounds.map((round, index) => {
          const prefix = `rounds.${round.key}`
          return (
            <fieldset className="round-editor" key={round.key}>
              <legend>Runde {index + 1}</legend>
              {props.rounds.length > 1 && (
                <button className="icon-button remove-round" type="button" aria-label={`Fjern runde ${index + 1}`} onClick={() => props.onRemove(round.key)}>
                  <Trash2 aria-hidden="true" />
                </button>
              )}
              <label>
                <span>Navn</span>
                <input required value={round.name} aria-invalid={Boolean(props.errors[`${prefix}.name`])} aria-describedby={`${round.key}-name-error`} onChange={(event) => props.onChange(round.key, { name: event.target.value })} />
                <FieldError id={`${round.key}-name-error`}>{props.errors[`${prefix}.name`]}</FieldError>
              </label>
              <label>
                <span>Dato</span>
                <input type="date" min={props.tournament.startDate} max={props.tournament.endDate} required value={round.date} aria-invalid={Boolean(props.errors[`${prefix}.date`])} aria-describedby={`${round.key}-date-error`} onChange={(event) => props.onChange(round.key, { date: event.target.value })} />
                <FieldError id={`${round.key}-date-error`}>{props.errors[`${prefix}.date`]}</FieldError>
              </label>
              <label>
                <span>Spilleform</span>
                <select value={round.scoringFormat} onChange={(event) => {
                  if (isScoringFormat(event.target.value)) props.onChange(round.key, { scoringFormat: event.target.value })
                }}>
                  <option value="singles_match_play">Matchspill (singel, 18 hull)</option>
                  <option value="individual_stableford">Stableford (individuelt, 18 hull)</option>
                  <option value="individual_stroke_play">Individuell slagkonkurranse</option>
                  <option value="team_scramble">Lagscramble (to spillere)</option>
                  <option value="two_player_foursomes">Foursomes (to spillere)</option>
                  <option value="four_ball_stroke_play">Four-ball (to spillere, 18 hull)</option>
                </select>
              </label>
            </fieldset>
          )
        })}
      </div>
      <button className="button add-round" type="button" onClick={props.onAdd} disabled={props.rounds.length >= 30}>
        <Plus aria-hidden="true" /> {props.rounds.length >= 30 ? 'Maks 30 runder' : 'Legg til runde'}
      </button>
      <WizardControls back={props.onBack} next={props.onNext} />
    </section>
  )
}
