import { ArrowLeft, ArrowRight } from 'lucide-react'

interface WizardControlsProps {
  back?: () => void
  next?: () => void
  submit?: boolean
  submitting?: boolean
}

export function WizardControls({ back, next, submit = false, submitting = false }: WizardControlsProps) {
  return (
    <div className="wizard-controls">
      {back ? (
        <button className="button secondary" type="button" onClick={back} disabled={submitting}>
          <ArrowLeft aria-hidden="true" /> Tilbake
        </button>
      ) : <span />}
      {submit ? (
        <button className="button primary" type="submit" disabled={submitting}>
          {submitting ? 'Oppretter …' : 'Opprett turnering'}
        </button>
      ) : (
        <button className="button primary" type="button" onClick={next}>
          Neste <ArrowRight aria-hidden="true" />
        </button>
      )}
    </div>
  )
}

export function FieldError({ id, children }: { id: string; children?: string }) {
  if (!children) return null
  return <span className="field-error" id={id}>{children}</span>
}
