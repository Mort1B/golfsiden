import { decodeArray, decodeBoolean, decodeInteger, decodeObject, decodeString, decodeTimestamp, decodeUuid, invalidData } from '../decoder'
import { decodeScoreRevision } from '../scorecards/revision'
import { decodeScoreVisibility } from '../visibility'
import type { FourBallEntry, FourBallInput, FourBallPartner, FourBallPlayer, FourBallReadEntry, FourBallReadCard, FourBallScoringCard, FourBallSelected } from './contracts'

const fail = (path: string): never => invalidData('four-ball-scorekort', path)
const integer = (value: unknown, path: string, min?: number, max?: number) => decodeInteger(value, path, min, max)
const nullableInt = (value: unknown, path: string) => value === null ? null : integer(value, path)
function exact(data: Record<string, unknown>, keys: string[], path: string): void {
  if (Object.keys(data).length !== keys.length || keys.some(key => !(key in data))) fail(path)
}
export function decodeFourBallInput(value: unknown, path = 'input'): FourBallInput {
  const data = decodeObject(value, path)
  if (data.type === 'no_score') { exact(data, ['type'], path); return { type: 'no_score' } }
  if (data.type === 'numeric') {
    exact(data, ['type', 'gross_strokes'], path)
    return { type: 'numeric', gross_strokes: integer(data.gross_strokes, path, 1, 20) }
  }
  return fail(path)
}
function readEntry(value: unknown, path: string): FourBallReadEntry {
  const data = decodeObject(value, path)
  exact(data, ['id', 'input'], path)
  return { id: decodeUuid(data.id, path), input: decodeFourBallInput(data.input, path) }
}
function entry(value: unknown, path: string, round: string, hole: string, player: string): FourBallEntry {
  const data = decodeObject(value, path)
  exact(data, ['id', 'input', 'revision', 'round_id', 'hole_id', 'owner', 'submitted_by', 'submitted_at', 'updated_at'], path)
  const owner = decodeObject(data.owner, `${path}.owner`)
  exact(owner, ['type', 'id'], path)
  if (owner.type !== 'player' || owner.id !== player || data.round_id !== round || data.hole_id !== hole) fail(path)
  return { id: decodeUuid(data.id, path), input: decodeFourBallInput(data.input, path),
    revision: decodeScoreRevision(data.revision), round_id: round, hole_id: hole, owner: { type: 'player', id: player },
    submitted_by: decodeUuid(data.submitted_by, path), submitted_at: decodeTimestamp(data.submitted_at, path), updated_at: decodeTimestamp(data.updated_at, path) }
}
function pair<T>(values: T[], path: string): [T, T] {
  const [first, second] = values
  if (values.length !== 2 || first === undefined || second === undefined) return fail(path)
  return [first, second]
}
function partner(value: unknown, path: string): FourBallPartner {
  const data = decodeObject(value, path)
  return { player_id: decodeUuid(data.player_id, path), display_name: decodeString(data.display_name, path), playing_handicap: integer(data.playing_handicap, path, -32768, 32767) }
}
function selected(value: unknown, path: string, players: FourBallPlayer[], metric: 'gross' | 'net'): FourBallSelected | null {
  const scores = players.flatMap(player => player.score?.input.type === 'numeric'
    ? [{ id: player.player_id, strokes: metric === 'gross' ? player.score.input.gross_strokes : player.net_strokes }] : [])
  if (scores.length === 0) { if (value !== null) fail(path); return null }
  const data = decodeObject(value, path)
  const strokes = integer(data.strokes, path)
  const ids = decodeArray(data.player_ids, path, decodeUuid)
  const minimum = Math.min(...scores.map(score => score.strokes ?? fail(path)))
  const winners = scores.filter(score => score.strokes === minimum).map(score => score.id)
  if (strokes !== minimum || ids.length !== winners.length || new Set(ids).size !== ids.length || winners.some(id => !ids.includes(id))) fail(path)
  return { strokes, player_ids: ids }
}
function decodeCard<E extends FourBallReadEntry>(value: unknown, round: string, team: string,
  decodeEntry: (value: unknown, path: string, round: string, hole: string, player: string) => E) {
  const data = decodeObject(value, 'card')
  const owner = decodeObject(data.owner, 'card.owner')
  if (data.round_id !== round || owner.type !== 'team' || owner.id !== team) fail('card.identity')
  const partners = pair(decodeArray(data.partners, 'partners', partner), 'partners')
  if (partners[0].player_id === partners[1].player_id) fail('partners.identity')
  const visibility = decodeScoreVisibility(data.visibility, 'visibility', 'four-ball-scorekort')
  const restricted = visibility.mode === 'front_nine'
  const visible = restricted ? 9 : 18
  if (data.number_of_holes !== 18 || data.visible_hole_count !== visible) fail('card.holes')
  const holes = decodeArray(data.holes, 'holes', (value, path) => {
    const hole = decodeObject(value, path)
    const hole_id = decodeUuid(hole.hole_id, path)
    const hole_number = integer(hole.hole_number, path, 1, visible)
    const stroke_index = integer(hole.stroke_index, path, 1, 18)
    const players = pair(decodeArray(hole.players, `${path}.players`, (value, p): FourBallPlayer<E> => {
      const player = decodeObject(value, p)
      const player_id = decodeUuid(player.player_id, p)
      const identity = partners.find(item => item.player_id === player_id)
      if (!identity) return fail(p)
      const handicap_strokes = integer(player.handicap_strokes, p)
      const magnitude = Math.abs(identity.playing_handicap)
      // Positive handicaps receive strokes from SI 1; plus handicaps give back from SI 18.
      const allocated = Math.sign(identity.playing_handicap) * (Math.floor(magnitude / 18)
        + Number(identity.playing_handicap < 0 ? stroke_index > 18 - magnitude % 18 : stroke_index <= magnitude % 18))
      if (handicap_strokes !== allocated) fail(`${p}.handicap_strokes`)
      const score = player.score === null ? null : decodeEntry(player.score, `${p}.score`, round, hole_id, player_id)
      const net_strokes = nullableInt(player.net_strokes, p)
      if (net_strokes !== (score?.input.type === 'numeric' ? score.input.gross_strokes - handicap_strokes : null)) fail(`${p}.net`)
      return { player_id, handicap_strokes, score, net_strokes }
    }), path)
    if (players[0].player_id === players[1].player_id) fail(path)
    return { hole_id, hole_number, par: integer(hole.par, path, 1, 10), stroke_index, players,
      gross: selected(hole.gross, `${path}.gross`, players, 'gross'), net: selected(hole.net, `${path}.net`, players, 'net') }
  })
  if (holes.length !== visible || new Set(holes.map(h => h.hole_number)).size !== visible
    || new Set(holes.map(h => h.hole_id)).size !== visible || new Set(holes.map(h => h.stroke_index)).size !== visible) fail('holes.identity')
  const scored = holes.filter(h => h.gross !== null)
  const gross_total = nullableInt(data.gross_total, 'gross_total')
  const net_total = nullableInt(data.net_total, 'net_total')
  const par_played = integer(data.par_played, 'par_played', 0)
  if (data.holes_scored !== scored.length || gross_total !== (scored.length ? scored.reduce((n, h) => n + (h.gross?.strokes ?? 0), 0) : null)
    || net_total !== (scored.length ? scored.reduce((n, h) => n + (h.net?.strokes ?? 0), 0) : null)
    || par_played !== scored.reduce((n, h) => n + h.par, 0)) fail('totals')
  const complete = data.complete === null ? null : decodeBoolean(data.complete, 'complete')
  const confirmed = data.confirmed === null ? null : decodeBoolean(data.confirmed, 'confirmed')
  const confirmed_at = data.confirmed_at === null ? null : decodeTimestamp(data.confirmed_at, 'confirmed_at')
  if (restricted ? complete !== null || confirmed !== null || confirmed_at !== null
    : complete !== (scored.length === 18) || confirmed === null || (confirmed && !complete) || confirmed !== (confirmed_at !== null)) fail('completion')
  return { format: 'four_ball_stroke_play' as const, round_id: round, owner: { type: 'team' as const, id: team },
    owner_name: decodeString(data.owner_name, 'owner_name'), partners, holes, gross_total, net_total, par_played,
    holes_scored: scored.length, number_of_holes: 18, visible_hole_count: visible, complete, confirmed, confirmed_at, visibility }
}
export function decodeFourBallRead(value: unknown, round: string, team: string): FourBallReadCard {
  return { ...decodeCard(value, round, team, readEntry), projection: 'read' }
}
export function decodeFourBallScoring(value: unknown, round: string, team: string): FourBallScoringCard {
  const card = decodeCard(value, round, team, entry)
  if (card.visibility.mode !== 'full') fail('scoring.visibility')
  return { ...card, projection: 'scoring' }
}
