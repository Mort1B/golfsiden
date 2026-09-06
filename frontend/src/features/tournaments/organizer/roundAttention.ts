import type { PairingValidation } from '../../../api/roundLifecycle'
import type { RoundCompletionValidation } from '../../../api/scorecards'
import type { Round, Tournament } from '../../../api/types'
import { scoringSearch } from '../../scoring/selection'
import { lifecycleReady, readinessDestination, roundManagementUrl } from '../lifecycle/lifecycleState'

export interface RoundAttention { text: string; href: string; link: string }

export function openingAttention(tournament: Tournament, round: Round, validation: PairingValidation): RoundAttention[] {
  const lifecycle = roundManagementUrl(tournament.id, round.id)
  const label = `Runde ${round.round_number}`
  if (tournament.status === 'active' && lifecycleReady(round, validation)) {
    return [{ text: `${label} er klar til å åpnes.`, href: lifecycle, link: 'Se åpning' }]
  }
  const items: RoundAttention[] = []
  const missing = validation.missing_flight_players.length
  if (missing) items.push({ text: `${label} har ${missing} ${missing === 1 ? 'spiller' : 'spillere'} uten flight.`,
    href: roundManagementUrl(tournament.id, round.id, 'pairings'), link: 'Fordel i flighter' })
  const destinations = new Set(validation.issues.filter(issue => issue.code !== 'missing_flight_assignment').map(issue => readinessDestination(issue.code)))
  if (tournament.status !== 'active') destinations.add('lifecycle')
  for (const destination of destinations) {
    const message = destination === 'courses' ? 'trenger baneoppsett'
      : destination === 'pairings' ? 'trenger kontroll av spillegruppene'
        : destination === 'invitations' ? 'trenger aktive deltakere' : 'trenger kontroll av turnerings- og rundestatus'
    items.push({ text: `${label} ${message}.`, href: roundManagementUrl(tournament.id, round.id, destination),
      link: destination === 'courses' ? 'Se baneoppsett' : destination === 'pairings' ? 'Se spillegrupper'
        : destination === 'invitations' ? 'Se invitasjoner' : 'Se rundestyring' })
  }
  return items.length ? items : [{ text: `${label} er ikke klar til å åpnes.`, href: lifecycle, link: 'Se krav før åpning' }]
}

export function completionAttention(round: Round, validation: RoundCompletionValidation): RoundAttention[] {
  const href = roundManagementUrl(round.tournament_id, round.id)
  const label = `Runde ${round.round_number}`
  if (lifecycleReady(round, undefined, validation)) {
    return [{ text: `${label} er klar til å ${round.status === 'open' ? 'fullføres' : 'låses'}.`, href, link: 'Se rundestyring' }]
  }
  const incomplete = validation.owners.filter(owner => owner.complete === false).length
  const awaiting = validation.owners.filter(owner => owner.complete === true && owner.confirmed === false)
  const items: RoundAttention[] = []
  if (incomplete) items.push({ text: `${label} har ${incomplete} ${incomplete === 1 ? 'ufullstendig' : 'ufullstendige'} scorekort.`, href, link: 'Se scorekort' })
  if (awaiting.length) {
    const only = awaiting.length === 1 ? awaiting[0] : undefined
    items.push({ text: `${label}: ${awaiting.length} scorekort må bekreftes.`,
      href: only ? `/score?${scoringSearch({ tournamentId: round.tournament_id, roundId: round.id, owner: only.owner, view: 'summary' })}` : href,
      link: only ? `Bekreft scorekort for ${only.owner_name}` : 'Se kort som må bekreftes' })
  }
  return items.length ? items : [{ text: `${label} trenger kontroll før ${round.status === 'open' ? 'fullføring' : 'låsing'}.`, href, link: 'Se krav og scorekort' }]
}
