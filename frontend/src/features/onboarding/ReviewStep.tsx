import { CalendarDays, Flag, UserRound } from 'lucide-react'
import type { WizardDraft } from './wizardState'
import { WizardControls } from './WizardControls'
import type { RefObject } from 'react'
import { formatHandicap, parseHandicap } from '../handicap/format'
import type { ScoringFormat } from '../../api/types'

const formatLabel = {
  individual_stroke_play: 'Individuell slagkonkurranse',
  team_scramble: 'Lagscramble',
  two_player_foursomes: 'Foursomes (to spillere)',
} satisfies Record<ScoringFormat, string>

export function ReviewStep({ draft, onBack, submitting, headingRef }: { draft: WizardDraft; onBack: () => void; submitting: boolean; headingRef: RefObject<HTMLHeadingElement | null> }) {
  const handicap = parseHandicap(draft.creator.handicap)
  const mandatoryRound = draft.rounds.find((round) => round.key === draft.mandatoryRoundKey)
  return (
    <section className="wizard-step review-step" aria-labelledby="review-step-heading">
      <header><p className="eyebrow">Steg 4 av 4</p><h1 id="review-step-heading" ref={headingRef} tabIndex={-1}>Kontroller opplysningene</h1><p>Se over detaljene før du oppretter turneringen.</p></header>
      <dl className="review-summary">
        <div><dt><Flag aria-hidden="true" /> Turnering</dt><dd><strong>{draft.tournament.name.trim()}</strong><span>{draft.tournament.startDate} – {draft.tournament.endDate}</span></dd></div>
        <div><dt><UserRound aria-hidden="true" /> Administrator</dt><dd><strong>{draft.creator.displayName.trim()}</strong><span>@{draft.creator.username.trim().toLowerCase()} · HCP {handicap.ok ? formatHandicap(handicap.value) : draft.creator.handicap}</span></dd></div>
        <div><dt><CalendarDays aria-hidden="true" /> Runder</dt><dd><strong>{draft.rounds.length} planlagt</strong><span>Beste {draft.countedRounds} av {draft.rounds.length}</span><span>Obligatorisk: {mandatoryRound?.name.trim() || 'Ingen'}</span></dd></div>
      </dl>
      <ol className="review-rounds">
        {draft.rounds.map((round, index) => <li key={round.key}><span>{index + 1}</span><div><strong>{round.name.trim()}</strong><small>{round.date} · {formatLabel[round.scoringFormat]}</small></div></li>)}
      </ol>
      <WizardControls back={onBack} submit submitting={submitting} />
    </section>
  )
}
