import { decodeArray, decodeInteger, decodeObject, decodeString, decodeUuid, invalidData } from '../decoder'
import { decodeCard, nullableBoolean, status } from './cardDecoder'
import { exact } from './eventDecoder'
import type { MatchCompletion, MatchListing, MatchPlayerListing, MatchTable } from './contracts'
export function decodeListing(value: unknown, round: string): MatchListing {
  const d = decodeObject(value, 'listing'); exact(d, ['round_id', 'matches', 'writable_match_ids'])
  const matches = decodeArray(d.matches, 'matches', v => decodeCard(v, round, decodeUuid(decodeObject(v, 'match').match_id, 'match_id')))
  const writable = decodeArray(d.writable_match_ids, 'writable', decodeUuid)
  if (d.round_id !== round || new Set(matches.map(m => m.match_id)).size !== matches.length || new Set(writable).size !== writable.length || writable.some(id => !matches.some(m => m.match_id === id))) invalidData('matchdata', 'listing.identity')
  return { round_id: round, matches, writable_match_ids: writable }
}
export function decodePlayerListing(value: unknown, round: string, player: string): MatchPlayerListing {
  const d = decodeObject(value, 'player listing'); exact(d, ['round_id', 'player_id', 'matches', 'writable_match_ids'])
  if (decodeUuid(d.player_id, 'player') !== player) invalidData('matchdata', 'listing.player')
  const listing = decodeListing({ round_id: d.round_id, matches: d.matches, writable_match_ids: d.writable_match_ids }, round)
  if (listing.matches.length > 1 || listing.matches.some(card => !card.opponents.some(p => p.player_id === player))) invalidData('matchdata', 'listing.player matches')
  return { ...listing, player_id: player }
}
export function decodeCompletion(value: unknown, round: string): MatchCompletion {
  const d = decodeObject(value, 'completion'); exact(d, ['format', 'round_id', 'status', 'matches', 'ready_to_complete', 'ready_to_lock'])
  if (d.format !== 'singles_match_play' || d.round_id !== round) invalidData('matchdata', 'completion.identity')
  const matches = decodeArray(d.matches, 'matches', v => { const m = decodeObject(v, 'match'); exact(m, ['match_id', 'terminal', 'confirmed']); return { match_id: decodeUuid(m.match_id, 'match_id'), terminal: nullableBoolean(m.terminal), confirmed: nullableBoolean(m.confirmed) } })
  if (new Set(matches.map(m => m.match_id)).size !== matches.length || matches.some(m => m.confirmed === true && m.terminal !== true)) invalidData('matchdata', 'completion.matches')
  return { format: 'singles_match_play', round_id: round, status: status(d.status), matches, ready_to_complete: nullableBoolean(d.ready_to_complete), ready_to_lock: nullableBoolean(d.ready_to_lock) }
}
export function decodeTable(value: unknown, tournament: string): MatchTable {
  const d = decodeObject(value, 'table'); exact(d, ['tournament_id', 'entries'])
  if (d.tournament_id !== tournament) invalidData('matchdata', 'table.identity')
  const entries = decodeArray(d.entries, 'entries', v => {
    const e = decodeObject(v, 'entry'); exact(e, ['player_id', 'display_name', 'half_points', 'played', 'wins', 'draws', 'losses', 'position'])
    const entry = { player_id: decodeUuid(e.player_id, 'player'), display_name: decodeString(e.display_name, 'name'), half_points: decodeInteger(e.half_points, 'points', 0, 4294967295), played: decodeInteger(e.played, 'played', 0), wins: decodeInteger(e.wins, 'wins', 0), draws: decodeInteger(e.draws, 'draws', 0), losses: decodeInteger(e.losses, 'losses', 0), position: e.position === null ? null : decodeInteger(e.position, 'position', 1) }
    if (entry.played !== entry.wins + entry.draws + entry.losses || entry.half_points !== entry.wins * 2 + entry.draws || (entry.played === 0) !== (entry.position === null)) invalidData('matchdata', 'table.counts')
    return entry
  })
  if (new Set(entries.map(e => e.player_id)).size !== entries.length) invalidData('matchdata', 'table.duplicate')
  for (const entry of entries) if (entry.position !== null && entry.position !== 1 + entries.filter(e => e.played > 0 && e.half_points > entry.half_points).length) invalidData('matchdata', 'table.rank')
  return { tournament_id: tournament, entries }
}
