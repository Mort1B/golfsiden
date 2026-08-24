import { Plus, Trash2 } from 'lucide-react'
import type { FieldErrors } from './validation'
import type { RoundDraft, TournamentDraft } from './wizardState'
import { FieldError, WizardControls } from './WizardControls'
import type { RefObject } from 'react'
import { isScoringFormat } from '../../api/scoringFormats'

interface RoundsStepProps {
  tournament: TournamentDraft
  rounds: RoundDraft[]
  errors: FieldErrors
  onAdd: () => void
  onRemove: (key: string) => void
  onChange: (key: string, value: Partial<Omit<RoundDraft, 'key'>>) => void
  onBack: () => void
  onNext: () => void
  headingRef: RefObject<HTMLHeadingElement | null>
}

export function RoundsStep(props: RoundsStepProps) {
  return (
    <section className="wizard-step" aria-labelledby="rounds-step-heading">
      <header><p className="eyebrow">Steg 2 av 4</p><h1 id="rounds-step-heading" ref={props.headingRef} tabIndex={-1}>Planlegg rundene</h1><p>Velg spilleform for hver runde. Bane og utslagssted kan settes senere.</p></header>
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
                  <option value="individual_stroke_play">Individuell slagkonkurranse</option>
                  <option value="team_scramble">Lagscramble (to spillere)</option>
                  <option value="two_player_foursomes">Foursomes (to spillere)</option>
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
