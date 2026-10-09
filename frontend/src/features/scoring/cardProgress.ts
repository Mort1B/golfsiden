import type { ScoringScorecard } from '../../api/scorecards'
import type { FourBallInput, FourBallScoringCard } from '../../api/fourBall'
import type { StablefordScoringCard } from '../../api/stableford'
import type { PendingScore } from './offline/model'
import type { LocalDraft } from './recovery/store'

export type ProgressCard = ScoringScorecard | FourBallScoringCard | StablefordScoringCard
// Display only: never writes a card, computes totals or authorizes confirmation.
export function cardProgress(card: ProgressCard, account: string, tournament: string, pending: readonly PendingScore[], drafts: readonly LocalDraft[]) {
  const format = 'format' in card ? card.format : 'stroke'
  const edits = new Map<string, number | FourBallInput>()
  const apply = (target: LocalDraft['target'], value: number | FourBallInput) => {
    if (target.accountId !== account || target.tournamentId !== tournament || target.roundId !== card.round_id) return
    const protocol = 'protocol' in target ? target.protocol : undefined
    const matches = format === 'four_ball_stroke_play' ? protocol === 'four_ball_v1' && 'sideId' in target && target.sideId === card.owner.id
      : format === 'individual_stableford' ? protocol === 'stableford_v1' && target.owner.id === card.owner.id
      : protocol === undefined && target.owner.type === card.owner.type && target.owner.id === card.owner.id
    if (matches) edits.set(`${target.holeId}:${target.owner.id}`, value)
  }
  for (const item of pending) apply(item, item.desired)
  for (const draft of drafts) apply(draft.target, draft.value)
  const entered = new Set<string>(), verified = new Set<string>(), changed = new Set<string>()
  for (const hole of card.holes) {
    const entries = 'players' in hole ? hole.players.map(player => ({id:player.player_id,value:player.score?.input??null}))
      : [{id:card.owner.id,value:hole.score ? 'input' in hole.score ? hole.score.input : hole.score.gross_strokes : null}]
    const counts = (value: number | FourBallInput | null) => value !== null && (format !== 'four_ball_stroke_play' || typeof value === 'number' || value.type === 'numeric')
    const dirty = entries.some(entry => edits.has(`${hole.hole_id}:${entry.id}`))
    if (dirty) changed.add(hole.hole_id)
    if (!dirty && entries.some(entry => counts(entry.value))) verified.add(hole.hole_id)
    if (entries.some(entry => counts(edits.get(`${hole.hole_id}:${entry.id}`) ?? entry.value))) entered.add(hole.hole_id)
  }
  return {entered:entered.size,verified:verified.size,pending:changed.size,total:card.number_of_holes}
}
