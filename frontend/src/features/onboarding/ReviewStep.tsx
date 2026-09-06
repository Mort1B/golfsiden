import { CalendarDays, Flag, UserRound } from 'lucide-react'
import type { WizardDraft } from './wizardState'
import { WizardControls } from './WizardControls'
import type { RefObject } from 'react'
import { formatHandicap, parseHandicap } from '../handicap/format'
import type { ScoringFormat } from '../../api/types'
import type { AuthSession } from '../../api/auth'
import { MANDATORY_ROUND_EXPLANATION } from '../leaderboards/resultExplanations'

const formatLabel = {
  individual_stroke_play: 'Individuell slagkonkurranse',
  team_scramble: 'Lagscramble',
  two_player_foursomes: 'Foursomes (to spillere)',
} satisfies Record<ScoringFormat, string>

export function ReviewStep({ draft, onBack, submitting, headingRef, existingSession, backDisabled = false }: { draft: WizardDraft; onBack: () => void; submitting: boolean; headingRef: RefObject<HTMLHeadingElement | null>; existingSession?: AuthSession; backDisabled?: boolean }) {
  const handicap = parseHandicap(draft.creator.handicap)
  const mandatoryRound = draft.rounds.find((round) => round.key === draft.mandatoryRoundKey)
  return (
    <section className="wizard-step review-step" aria-labelledby="review-step-heading">
      <header><p className="eyebrow">Steg {existingSession ? '3 av 3' : '4 av 4'}</p><h1 id="review-step-heading" ref={headingRef} tabIndex={-1}>Kontroller opplysningene</h1><p>Se over detaljene før du oppretter turneringen.</p></header>
      <dl className="review-summary">
        <div><dt><Flag aria-hidden="true" /> Turnering</dt><dd><strong>{draft.tournament.name.trim()}</strong><span>{draft.tournament.startDate} – {draft.tournament.endDate}</span></dd></div>
        <div><dt><UserRound aria-hidden="true" /> Administrator</dt><dd><strong>{existingSession?.display_name ?? draft.creator.displayName.trim()}</strong><span>@{existingSession?.username ?? draft.creator.username.trim().toLowerCase()}{!existingSession && ` · HCP ${handicap.ok ? formatHandicap(handicap.value) : draft.creator.handicap}`}</span></dd></div>
        <div><dt><CalendarDays aria-hidden="true" /> Runder</dt><dd><strong>{draft.rounds.length} planlagt</strong><span>Beste {draft.countedRounds} av {draft.rounds.length}</span><span>Obligatorisk: {mandatoryRound?.name.trim() || 'Ingen'}</span></dd></div>
      </dl>
      {mandatoryRound && <p>{MANDATORY_ROUND_EXPLANATION}</p>}
      {existingSession && <p>Du blir administrator med den eksisterende kontoen din. Har kontoen en aktiv tilknyttet spiller, meldes spilleren på med gjeldende handicap som fast turneringshandicap. Ellers opprettes turneringen uten deg som deltaker. Bane, spillegrupper og invitasjoner ordnes i administrasjonen etterpå.</p>}
      <ol className="review-rounds">
        {draft.rounds.map((round, index) => <li key={round.key}><span>{index + 1}</span><div><strong>{round.name.trim()}</strong><small>{round.date} · {formatLabel[round.scoringFormat]}</small></div></li>)}
      </ol>
      <WizardControls back={onBack} submit submitting={submitting} backDisabled={backDisabled} />
    </section>
  )
}
