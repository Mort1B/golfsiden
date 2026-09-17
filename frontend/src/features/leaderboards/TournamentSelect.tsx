import type { Tournament } from '../../api/types'

interface Props {
  tournaments: Tournament[]
  tournamentId: string
  onChange: (id: string) => void
}

export function TournamentSelect({ tournaments, tournamentId, onChange }: Props) {
  return <label className="leaderboard-select">
    <span>Turnering</span>
    <select value={tournamentId} onChange={(event) => onChange(event.target.value)}>
      {tournaments.map((tournament) => (
        <option key={tournament.id} value={tournament.id}>{tournament.name}</option>
      ))}
    </select>
  </label>
}
