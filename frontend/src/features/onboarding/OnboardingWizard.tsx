import { useEffect, useRef, useState, type FormEvent } from 'react'
import { CreatorStep } from './CreatorStep'
import { OnboardingSuccessView } from './OnboardingSuccessView'
import { ReviewStep } from './ReviewStep'
import { RoundsStep } from './RoundsStep'
import { TournamentStep } from './TournamentStep'
import { hasErrors, validateAll, validateCreator, validateRounds, validateTournament, type FieldErrors } from './validation'
import { addRound, createInitialDraft, localDateString, removeRound, updateRound, type WizardDraft } from './wizardState'
import { useOnboardingSubmission } from './useOnboardingSubmission'

type WizardStep = 0 | 1 | 2 | 3
const stepLabels = ['Turnering', 'Runder', 'Spillerkonto', 'Kontroller']

function stepForErrors(errors: FieldErrors): WizardStep {
  const paths = Object.keys(errors)
  if (paths.some((path) => path.startsWith('tournament.'))) return 0
  if (paths.some((path) => path === 'rounds' || path.startsWith('rounds.'))) return 1
  return 2
}

export function OnboardingWizard({ onCreated }: { onCreated: () => void }) {
  const today = localDateString()
  const [draft, setDraft] = useState<WizardDraft>(() => createInitialDraft(today))
  const [step, setStep] = useState<WizardStep>(0)
  const [errors, setErrors] = useState<FieldErrors>({})
  const submission = useOnboardingSubmission(onCreated)
  const submissionAlert = useRef<HTMLDivElement>(null)
  const heading = useRef<HTMLHeadingElement>(null)
  const form = useRef<HTMLFormElement>(null)
  const focusInvalid = useRef(false)

  useEffect(() => {
    if (submission.state.status === 'error') submissionAlert.current?.focus()
  }, [submission.state.status])

  useEffect(() => {
    if (focusInvalid.current) {
      form.current?.querySelector<HTMLElement>('[aria-invalid="true"]')?.focus()
      focusInvalid.current = false
      return
    }
    heading.current?.focus()
  }, [errors, step])

  if (submission.state.success) return <OnboardingSuccessView success={submission.state.success} />

  const advance = (next: WizardStep) => {
    const currentErrors = step === 0
      ? validateTournament(draft.tournament, today)
      : step === 1
        ? validateRounds(draft.rounds, draft.tournament)
        : validateCreator(draft.creator)
    setErrors(currentErrors)
    if (!hasErrors(currentErrors)) {
      setStep(next)
      window.scrollTo({ top: 0, behavior: 'smooth' })
    } else {
      focusInvalid.current = true
    }
  }

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const allErrors = validateAll(draft, today)
    setErrors(allErrors)
    if (hasErrors(allErrors)) {
      focusInvalid.current = true
      setStep(stepForErrors(allErrors))
      return
    }
    const succeeded = await submission.submit(draft)
    if (succeeded) setDraft(createInitialDraft(today))
  }

  return (
    <main className="onboarding-page">
      <div className="onboarding-shell">
        <a className="brand onboarding-brand" href="/">Guttas Golf</a>
        <ol className="wizard-progress" aria-label="Fremdrift">
          {stepLabels.map((label, index) => <li key={label} className={index <= step ? 'reached' : ''} aria-current={index === step ? 'step' : undefined}><span>{index < 3 ? index + 1 : '✓'}</span><small>{label}</small></li>)}
        </ol>
        {submission.state.error && <div className="submission-error" role="alert" tabIndex={-1} ref={submissionAlert}>{submission.state.error}</div>}
        <form ref={form} onSubmit={(event) => void submit(event)} noValidate>
          {step === 0 && <TournamentStep headingRef={heading} value={draft.tournament} errors={errors} onChange={(tournament) => setDraft({ ...draft, tournament })} onNext={() => advance(1)} />}
          {step === 1 && <RoundsStep headingRef={heading} tournament={draft.tournament} rounds={draft.rounds} errors={errors} onAdd={() => setDraft(addRound(draft))} onRemove={(key) => setDraft(removeRound(draft, key))} onChange={(key, value) => setDraft(updateRound(draft, key, value))} onBack={() => setStep(0)} onNext={() => advance(2)} />}
          {step === 2 && <CreatorStep headingRef={heading} value={draft.creator} errors={errors} onChange={(creator) => setDraft({ ...draft, creator })} onBack={() => setStep(1)} onNext={() => advance(3)} />}
          {step === 3 && <ReviewStep headingRef={heading} draft={draft} onBack={() => setStep(2)} submitting={submission.state.status === 'submitting'} />}
        </form>
      </div>
    </main>
  )
}
