import type { Round, TournamentTieBreakPolicy } from '../../api/types'
export function finalMatchExplanation(rounds: Round[], finalRoundNumber: number, policy: TournamentTieBreakPolicy): string | null {
  return policy === 'final_round_score' && rounds.some(r => r.round_number === finalRoundNumber && r.scoring_format === 'singles_match_play')
    ? 'Siste planlagte runde er matchspill. Den gir ingen sammenlignbar sluttscore i brutto/netto; lik totalscore beholder delt plass.' : null
}
