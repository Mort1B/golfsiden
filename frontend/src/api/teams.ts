import {
  decodeArray,
  decodeInteger,
  decodeObject,
  decodeString,
  decodeUuid,
  invalidData,
} from './decoder'
import { requestDecoded } from './http'
import type { Team, TeamMember } from './types'

const timePattern = /^([01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d+)?$/

function decodeTime(value: unknown, path: string): string {
  const decoded = decodeString(value, path, 'lagdata')
  if (!timePattern.test(decoded)) invalidData('lagdata', path)
  return decoded
}

function decodeMember(value: unknown, path: string): TeamMember {
  const data = decodeObject(value, path, 'lagdata')
  return {
    player_id: decodeUuid(data.player_id, `${path}.player_id`, 'lagdata'),
    display_name: decodeString(data.display_name, `${path}.display_name`, 'lagdata'),
    display_order: data.display_order === null
      ? null
      : decodeInteger(data.display_order, `${path}.display_order`, 0, 32_767, 'lagdata'),
  }
}

function decodeTeam(value: unknown, path: string): Team {
  const data = decodeObject(value, path, 'lagdata')
  return {
    id: decodeUuid(data.id, `${path}.id`, 'lagdata'),
    round_id: decodeUuid(data.round_id, `${path}.round_id`, 'lagdata'),
    tournament_id: decodeUuid(data.tournament_id, `${path}.tournament_id`, 'lagdata'),
    name: decodeString(data.name, `${path}.name`, 'lagdata'),
    starting_hole: data.starting_hole === null
      ? null
      : decodeInteger(data.starting_hole, `${path}.starting_hole`, 1, 36, 'lagdata'),
    tee_time: data.tee_time === null ? null : decodeTime(data.tee_time, `${path}.tee_time`),
    members: decodeArray(data.members, `${path}.members`, decodeMember, 'lagdata'),
  }
}

export function decodeTeams(value: unknown, expectedRoundId: string, expectedTournamentId: string): Team[] {
  const teams = decodeArray(value, 'teams', decodeTeam, 'lagdata')
  const teamIds = new Set<string>()
  const assignedPlayerIds = new Set<string>()
  teams.forEach((team, teamIndex) => {
    const path = `teams[${teamIndex}]`
    if (team.round_id !== expectedRoundId || team.tournament_id !== expectedTournamentId) {
      invalidData('lagdata', `${path}.identity`)
    }
    if (teamIds.has(team.id)) invalidData('lagdata', `${path}.id duplicate`)
    teamIds.add(team.id)
    const playerIds = new Set<string>()
    team.members.forEach((member, memberIndex) => {
      if (playerIds.has(member.player_id)) invalidData('lagdata', `${path}.members[${memberIndex}].player_id duplicate`)
      if (assignedPlayerIds.has(member.player_id)) invalidData('lagdata', `${path}.members[${memberIndex}].player_id assigned twice`)
      playerIds.add(member.player_id)
      assignedPlayerIds.add(member.player_id)
    })
  })
  return teams
}

export const teamApi = {
  list: (roundId: string, tournamentId: string) => requestDecoded(
    `/api/rounds/${roundId}/teams`,
    (value) => decodeTeams(value, roundId, tournamentId),
  ),
}
