import { CalendarDays, Flag, UserRound } from 'lucide-react'
import type { WizardDraft } from './wizardState'
import { WizardControls } from './WizardControls'
import type { RefObject } from 'react'

const formatLabel: Record<string, string> = {
  individual_stroke_play: 'Individuell slagkonkurranse',
  team_scramble: 'Lagscramble',
}

export function ReviewStep({ draft, onBack, submitting, headingRef }: { draft: WizardDraft; onBack: () => void; submitting: boolean; headingRef: RefObject<HTMLHeadingElement | null> }) {
  return (
    <section className="wizard-step review-step" aria-labelledby="review-step-heading">
      <header><p className="eyebrow">Steg 4 av 4</p><h1 id="review-step-heading" ref={headingRef} tabIndex={-1}>Kontroller opplysningene</h1><p>Se over detaljene før du oppretter turneringen.</p></header>
      <dl className="review-summary">
        <div><dt><Flag aria-hidden="true" /> Turnering</dt><dd><strong>{draft.tournament.name.trim()}</strong><span>{draft.tournament.startDate} – {draft.tournament.endDate}</span></dd></div>
        <div><dt><UserRound aria-hidden="true" /> Administrator</dt><dd><strong>{draft.creator.displayName.trim()}</strong><span>{draft.creator.email.trim()}</span></dd></div>
        <div><dt><CalendarDays aria-hidden="true" /> Runder</dt><dd>{draft.rounds.length}</dd></div>
      </dl>
      <ol className="review-rounds">
        {draft.rounds.map((round, index) => <li key={round.key}><span>{index + 1}</span><div><strong>{round.name.trim()}</strong><small>{round.date} · {formatLabel[round.scoringFormat]}</small></div></li>)}
      </ol>
      <WizardControls back={onBack} submit submitting={submitting} />
    </section>
  )
}
