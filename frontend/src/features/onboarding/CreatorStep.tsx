import type { FieldErrors } from './validation'
import type { CreatorDraft } from './wizardState'
import { FieldError, WizardControls } from './WizardControls'
import type { RefObject } from 'react'

interface CreatorStepProps {
  value: CreatorDraft
  errors: FieldErrors
  onChange: (value: CreatorDraft) => void
  onBack: () => void
  onNext: () => void
  headingRef: RefObject<HTMLHeadingElement | null>
}

export function CreatorStep({ value, errors, onChange, onBack, onNext, headingRef }: CreatorStepProps) {
  return (
    <section className="wizard-step" aria-labelledby="creator-step-heading">
      <header><p className="eyebrow">Steg 3 av 4</p><h1 id="creator-step-heading" ref={headingRef} tabIndex={-1}>Din spillerkonto</h1><p>Du blir administrator og første spiller i turneringen.</p></header>
      <div className="form-fields">
        <label>
          <span>Visningsnavn</span>
          <input autoComplete="name" required value={value.displayName} aria-invalid={Boolean(errors['creator.displayName'])} aria-describedby="creator-name-error" onChange={(event) => onChange({ ...value, displayName: event.target.value })} />
          <FieldError id="creator-name-error">{errors['creator.displayName']}</FieldError>
        </label>
        <label>
          <span>Brukernavn</span>
          <input autoComplete="username" minLength={3} maxLength={32} pattern="[A-Za-z0-9_-]{3,32}" required value={value.username} aria-invalid={Boolean(errors['creator.username'])} aria-describedby="creator-username-help creator-username-error" onChange={(event) => onChange({ ...value, username: event.target.value })} />
          <small id="creator-username-help" className="field-help">3–32 bokstaver, tall, bindestrek eller understrek.</small>
          <FieldError id="creator-username-error">{errors['creator.username']}</FieldError>
        </label>
        <label>
          <span>Passord</span>
          <input type="password" autoComplete="new-password" minLength={12} required value={value.password} aria-invalid={Boolean(errors['creator.password'])} aria-describedby="creator-password-help creator-password-error" onChange={(event) => onChange({ ...value, password: event.target.value })} />
          <small id="creator-password-help" className="field-help">Minst 12 tegn. Mellomrom beholdes.</small>
          <FieldError id="creator-password-error">{errors['creator.password']}</FieldError>
        </label>
        <label>
          <span>Handicapindeks</span>
          <input type="text" inputMode="decimal" required value={value.handicap} aria-invalid={Boolean(errors['creator.handicap'])} aria-describedby="creator-handicap-help creator-handicap-error" onChange={(event) => onChange({ ...value, handicap: event.target.value })} />
          <small id="creator-handicap-help" className="field-help">Bruk komma eller punktum, for eksempel 14,4.</small>
          <FieldError id="creator-handicap-error">{errors['creator.handicap']}</FieldError>
        </label>
      </div>
      <WizardControls back={onBack} next={onNext} />
    </section>
  )
}
