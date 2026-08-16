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
          <span>E-post</span>
          <input type="email" inputMode="email" autoComplete="username" required value={value.email} aria-invalid={Boolean(errors['creator.email'])} aria-describedby="creator-email-error" onChange={(event) => onChange({ ...value, email: event.target.value })} />
          <FieldError id="creator-email-error">{errors['creator.email']}</FieldError>
        </label>
        <label>
          <span>Passord</span>
          <input type="password" autoComplete="new-password" minLength={12} required value={value.password} aria-invalid={Boolean(errors['creator.password'])} aria-describedby="creator-password-help creator-password-error" onChange={(event) => onChange({ ...value, password: event.target.value })} />
          <small id="creator-password-help" className="field-help">Minst 12 tegn. Mellomrom beholdes.</small>
          <FieldError id="creator-password-error">{errors['creator.password']}</FieldError>
        </label>
        <label>
          <span>Handicapindeks</span>
          <input type="number" inputMode="decimal" step="0.1" min="-10" max="54" required value={value.handicap} aria-invalid={Boolean(errors['creator.handicap'])} aria-describedby="creator-handicap-error" onChange={(event) => onChange({ ...value, handicap: event.target.value })} />
          <FieldError id="creator-handicap-error">{errors['creator.handicap']}</FieldError>
        </label>
      </div>
      <WizardControls back={onBack} next={onNext} />
    </section>
  )
}
