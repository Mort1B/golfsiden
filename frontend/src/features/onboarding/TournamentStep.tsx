import type { FieldErrors } from './validation'
import type { TournamentDraft } from './wizardState'
import { FieldError, WizardControls } from './WizardControls'
import type { RefObject } from 'react'

interface TournamentStepProps {
  value: TournamentDraft
  errors: FieldErrors
  onChange: (value: TournamentDraft) => void
  onNext: () => void
  headingRef: RefObject<HTMLHeadingElement | null>
}

export function TournamentStep({ value, errors, onChange, onNext, headingRef }: TournamentStepProps) {
  return (
    <section className="wizard-step" aria-labelledby="tournament-step-heading">
      <header><p className="eyebrow">Steg 1 av 4</p><h1 id="tournament-step-heading" ref={headingRef} tabIndex={-1}>Om turneringen</h1><p>Start med navn og datoer. Alt annet blir knyttet til denne turneringen.</p></header>
      <div className="form-fields">
        <label>
          <span>Turneringsnavn</span>
          <input required value={value.name} aria-invalid={Boolean(errors['tournament.name'])} aria-describedby="tournament-name-error" onChange={(event) => onChange({ ...value, name: event.target.value })} />
          <FieldError id="tournament-name-error">{errors['tournament.name']}</FieldError>
        </label>
        <label>
          <span>Beskrivelse <small>valgfritt</small></span>
          <textarea rows={4} value={value.description} aria-invalid={Boolean(errors['tournament.description'])} aria-describedby="tournament-description-error" onChange={(event) => onChange({ ...value, description: event.target.value })} />
          <FieldError id="tournament-description-error">{errors['tournament.description']}</FieldError>
        </label>
        <div className="date-fields">
          <label>
            <span>Startdato</span>
            <input type="date" required value={value.startDate} aria-invalid={Boolean(errors['tournament.startDate'])} aria-describedby="tournament-start-error" onChange={(event) => onChange({ ...value, startDate: event.target.value })} />
            <FieldError id="tournament-start-error">{errors['tournament.startDate']}</FieldError>
          </label>
          <label>
            <span>Sluttdato</span>
            <input type="date" required value={value.endDate} aria-invalid={Boolean(errors['tournament.endDate'])} aria-describedby="tournament-end-error" onChange={(event) => onChange({ ...value, endDate: event.target.value })} />
            <FieldError id="tournament-end-error">{errors['tournament.endDate']}</FieldError>
          </label>
        </div>
      </div>
      <WizardControls next={onNext} />
    </section>
  )
}
