import { useState } from 'react'
import { ChevronDown } from 'lucide-react'
import type { Round, ScoringFormat } from '../../../api/types'
import { StatusBadge } from '../../../ui/StatusBadge'
import { PairingEditor } from './PairingEditor'

interface Props { tournamentId: string; rounds: Round[] }

const pairingFormatLabels = {
  individual_stroke_play: 'Individuell · flighter',
  team_scramble: 'Scramble · lag og flighter',
  two_player_foursomes: 'Foursomes · lag og flighter',
} satisfies Record<ScoringFormat, string>

export function PairingSection({ tournamentId, rounds }: Props) {
  const [expandedRoundId, setExpandedRoundId] = useState<string | null>(null)
  return <div className="round-pairings">
    {rounds.map((round) => {
      const expanded = round.id === expandedRoundId
      return <article className="round-pairing-card" key={round.id}><header><div><strong>Runde {round.round_number}: {round.name}</strong><span>{pairingFormatLabels[round.scoring_format]}</span></div><StatusBadge status={round.status} /><button type="button" aria-expanded={expanded} aria-controls={`pairing-editor-${round.id}`} onClick={() => setExpandedRoundId((current) => current === round.id ? null : round.id)}>{expanded ? 'Lukk' : round.status === 'draft' ? 'Rediger' : 'Vis'}<ChevronDown aria-hidden="true" /></button></header><div id={`pairing-editor-${round.id}`} hidden={!expanded}><PairingEditor tournamentId={tournamentId} round={round} expanded={expanded} /></div></article>
    })}
  </div>
}
