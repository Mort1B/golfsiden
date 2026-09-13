import type { TournamentTieBreakPolicy } from '../../api/types'

export function tieBreakLabel(policy: TournamentTieBreakPolicy): string {
  return policy === 'shared_positions' ? 'Delt plass' : 'Siste runde, deretter delt plass'
}

export function tieBreakExplanation(policy: TournamentTieBreakPolicy): string {
  return policy === 'shared_positions'
    ? 'Lik totalscore og lik kvalifisering gir delt plass sammenlagt.'
    : 'Ved lik totalscore sammenlignes siste planlagte runde i valgt brutto-/nettovisning, også når runden ikke teller blant de beste. Alle i den like gruppen må være kvalifisert, uten tellende foreløpig score og med en fullført, synlig siste runde. Ellers beholdes delt plass. Lik score i siste runde gir fortsatt delt plass.'
}
