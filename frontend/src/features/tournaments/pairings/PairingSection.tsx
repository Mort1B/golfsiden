import { useState } from 'react'
import { ChevronDown } from 'lucide-react'
import type { Round } from '../../../api/types'
import { StatusBadge } from '../../../ui/StatusBadge'
import { PairingEditor } from './PairingEditor'

interface Props { tournamentId: string; rounds: Round[] }

export function PairingSection({ tournamentId, rounds }: Props) {
  const [expandedRoundId, setExpandedRoundId] = useState<string | null>(null)
  return <div className="round-pairings">
    {rounds.map((round) => {
      const expanded = round.id === expandedRoundId
      return <article className="round-pairing-card" key={round.id}><header><div><strong>Runde {round.round_number}: {round.name}</strong><span>{round.scoring_format === 'team_scramble' ? 'Scramble · lag og flighter' : 'Individuell · flighter'}</span></div><StatusBadge status={round.status} /><button type="button" aria-expanded={expanded} aria-controls={`pairing-editor-${round.id}`} onClick={() => setExpandedRoundId((current) => current === round.id ? null : round.id)}>{expanded ? 'Lukk' : round.status === 'draft' ? 'Rediger' : 'Vis'}<ChevronDown aria-hidden="true" /></button></header><div id={`pairing-editor-${round.id}`} hidden={!expanded}><PairingEditor tournamentId={tournamentId} round={round} expanded={expanded} /></div></article>
    })}
  </div>
}
