import type { PairingGroup, RoundPairings } from '../../api/pairings'
import { isTeamScoringFormat } from '../../api/scoringFormats'
import { EmptyState } from '../../ui/AsyncState'
import './roundDetails.css'

function Groups({ title, groups, schedule = false }: {
  title: string; groups: PairingGroup[]; schedule?: boolean
}) {
  return <section aria-label={title}>
    <div className="section-heading"><h2>{title}</h2><span>{groups.length}</span></div>
    {groups.length === 0 ? <EmptyState>Ingen {title.toLocaleLowerCase('nb-NO')} er satt opp.</EmptyState> :
      <div className="team-grid">{groups.map((group) => <article className="team-card round-group" key={group.id}>
        <header><h3>{group.name}</h3></header>
        {schedule && <p className="flight-schedule">
          <span>{group.tee_time ? <>Starttid <time>{group.tee_time.replace(/:00$/, '')}</time></> : 'Starttid ikke satt'}</span>
          <span>{group.starting_hole === null ? 'Starthull ikke satt' : `Start hull ${group.starting_hole}`}</span>
        </p>}
        {group.members.length === 0 ? <EmptyState>Ingen spillere er satt opp.</EmptyState> :
          <ol>{group.members.map((member) => <li key={member.player_id}>{member.display_name}</li>)}</ol>}
      </article>)}</div>}
  </section>
}

export function RoundGroups({ pairings }: { pairings: RoundPairings }) {
  return <>
    <Groups title="Flighter" groups={pairings.flights} schedule />
    {isTeamScoringFormat(pairings.scoring_format) && <>
      <p className="round-detail-note">Lagene har felles scorekort. Starttid og starthull vises på flighten.</p>
      <Groups title="Lag" groups={pairings.teams} />
    </>}
    {pairings.legacy_individual_groups.length > 0 && <>
      <p className="round-detail-note">Eldre grupper fra individuelt spill er ikke flighter eller lag med felles scorekort.</p>
      <Groups title="Eldre individuelle grupper" groups={pairings.legacy_individual_groups} />
    </>}
  </>
}
