import { decodeArray, decodeBoolean, decodeInteger, decodeObject, decodeTimestamp, decodeUuid, invalidData } from '../decoder'
import { decodeScoreVisibility } from '../visibility'
import type { ReadScoreEntry, ReadScorecard, ReadScorecardHole, ScoreEntry, ScoreOwner, ScoringScorecard, ScoringScorecardHole } from './contracts'
import { ownerEquals } from './contracts'

function invalid(path: string): never { return invalidData('scorekortdata', path) }

export function decodeScoreOwner(value: unknown, path: string): ScoreOwner {
  const data = decodeObject(value, path, 'scorekortdata')
  if (data.type === 'player') return { type: 'player', id: decodeUuid(data.id, `${path}.id`, 'scorekortdata') }
  if (data.type === 'team') return { type: 'team', id: decodeUuid(data.id, `${path}.id`, 'scorekortdata') }
  return invalid(`${path}.type`)
}

function rejectFields(data: Record<string, unknown>, path: string, fields: readonly string[]): void {
  for (const field of fields) if (Object.hasOwn(data, field)) invalid(`${path}.${field}`)
}

export function decodeScoreEntry(value: unknown, path: string, expectedRoundId: string, expectedHoleId: string, expectedOwner: ScoreOwner): ScoreEntry {
  const data = decodeObject(value, path, 'scorekortdata')
  const decoded: ScoreEntry = {
    id: decodeUuid(data.id, `${path}.id`, 'scorekortdata'),
    round_id: decodeUuid(data.round_id, `${path}.round_id`, 'scorekortdata'),
    hole_id: decodeUuid(data.hole_id, `${path}.hole_id`, 'scorekortdata'),
    owner: decodeScoreOwner(data.owner, `${path}.owner`),
    gross_strokes: decodeInteger(data.gross_strokes, `${path}.gross_strokes`, 1, 20, 'scorekortdata'),
    submitted_by: decodeUuid(data.submitted_by, `${path}.submitted_by`, 'scorekortdata'),
    submitted_at: decodeTimestamp(data.submitted_at, `${path}.submitted_at`, 'scorekortdata'),
    updated_at: decodeTimestamp(data.updated_at, `${path}.updated_at`, 'scorekortdata'),
  }
  if (decoded.round_id !== expectedRoundId || decoded.hole_id !== expectedHoleId || !ownerEquals(decoded.owner, expectedOwner)) invalid(`${path}.identity`)
  return decoded
}

function decodeReadScore(value: unknown, path: string): ReadScoreEntry {
  const data = decodeObject(value, path, 'scorekortdata')
  rejectFields(data, path, ['round_id', 'hole_id', 'owner', 'submitted_by', 'submitted_at', 'updated_at'])
  return { id: decodeUuid(data.id, `${path}.id`, 'scorekortdata'), gross_strokes: decodeInteger(data.gross_strokes, `${path}.gross_strokes`, 1, 20, 'scorekortdata') }
}

function holeFields(data: Record<string, unknown>, path: string) {
  return {
    hole_id: decodeUuid(data.hole_id, `${path}.hole_id`, 'scorekortdata'),
    hole_number: decodeInteger(data.hole_number, `${path}.hole_number`, 1, undefined, 'scorekortdata'),
    par: decodeInteger(data.par, `${path}.par`, 1, undefined, 'scorekortdata'),
    stroke_index: decodeInteger(data.stroke_index, `${path}.stroke_index`, 1, undefined, 'scorekortdata'),
  }
}

function decodeScoringHole(value: unknown, path: string, roundId: string, expectedOwner: ScoreOwner): ScoringScorecardHole {
  const data = decodeObject(value, path, 'scorekortdata')
  const fields = holeFields(data, path)
  const score = data.score === null ? null : decodeScoreEntry(data.score, `${path}.score`, roundId, fields.hole_id, expectedOwner)
  const net_strokes = data.net_strokes === null ? null : decodeInteger(data.net_strokes, `${path}.net_strokes`, undefined, undefined, 'scorekortdata')
  if ((score === null) !== (net_strokes === null)) invalid(`${path}.net_strokes`)
  return { ...fields, score, net_strokes }
}

function decodeReadHole(value: unknown, path: string): ReadScorecardHole {
  const data = decodeObject(value, path, 'scorekortdata')
  rejectFields(data, path, ['submitted_by', 'submitted_at', 'updated_at'])
  const score = data.score === null ? null : decodeReadScore(data.score, `${path}.score`)
  const net_strokes = data.net_strokes === null ? null : decodeInteger(data.net_strokes, `${path}.net_strokes`, undefined, undefined, 'scorekortdata')
  if ((score === null) !== (net_strokes === null)) invalid(`${path}.net_strokes`)
  return { ...holeFields(data, path), score, net_strokes }
}

function validateCard(roundId: string, decodedOwner: ScoreOwner, expectedRoundId: string, expectedOwner: ScoreOwner, holes: ReadScorecardHole[] | ScoringScorecardHole[], holesScored: number, grossTotal: number, netTotal: number, maximumStrokeIndex: number): void {
  const scored = holes.filter((hole) => hole.score !== null).length
  if (roundId !== expectedRoundId || !ownerEquals(decodedOwner, expectedOwner)
    || holes.some((hole, index) => hole.hole_number !== index + 1)
    || new Set(holes.map((hole) => hole.hole_id)).size !== holes.length
    || new Set(holes.flatMap((hole) => hole.score === null ? [] : [hole.score.id])).size !== scored
    || new Set(holes.map((hole) => hole.stroke_index)).size !== holes.length
    || holes.some((hole) => hole.stroke_index > maximumStrokeIndex) || holesScored !== scored
    || grossTotal !== holes.reduce((total, hole) => total + (hole.score?.gross_strokes ?? 0), 0)
    || netTotal !== holes.reduce((total, hole) => total + (hole.net_strokes ?? 0), 0)) invalid('scorecard.identity')
}

export function decodeScoringScorecard(value: unknown, expectedRoundId: string, expectedOwner: ScoreOwner): ScoringScorecard {
  const data = decodeObject(value, 'scorecard', 'scorekortdata')
  const holes = decodeArray(data.holes, 'scorecard.holes', (item, path) => decodeScoringHole(item, path, expectedRoundId, expectedOwner), 'scorekortdata')
  const decoded: ScoringScorecard = {
    projection: 'scoring', round_id: decodeUuid(data.round_id, 'scorecard.round_id', 'scorekortdata'),
    owner: decodeScoreOwner(data.owner, 'scorecard.owner'), holes,
    gross_total: decodeInteger(data.gross_total, 'scorecard.gross_total', 0, undefined, 'scorekortdata'),
    net_total: decodeInteger(data.net_total, 'scorecard.net_total', undefined, undefined, 'scorekortdata'),
    playing_handicap: decodeInteger(data.playing_handicap, 'scorecard.playing_handicap', undefined, undefined, 'scorekortdata'),
    holes_scored: decodeInteger(data.holes_scored, 'scorecard.holes_scored', 0, holes.length, 'scorekortdata'),
    number_of_holes: decodeInteger(data.number_of_holes, 'scorecard.number_of_holes', 1, undefined, 'scorekortdata'),
    complete: decodeBoolean(data.complete, 'scorecard.complete', 'scorekortdata'),
    confirmed: decodeBoolean(data.confirmed, 'scorecard.confirmed', 'scorekortdata'),
    confirmed_by: data.confirmed_by === null ? null : decodeUuid(data.confirmed_by, 'scorecard.confirmed_by', 'scorekortdata'),
    confirmed_at: data.confirmed_at === null ? null : decodeTimestamp(data.confirmed_at, 'scorecard.confirmed_at', 'scorekortdata'),
  }
  validateCard(decoded.round_id, decoded.owner, expectedRoundId, expectedOwner, holes, decoded.holes_scored, decoded.gross_total, decoded.net_total, decoded.number_of_holes)
  const confirmationConsistent = decoded.confirmed ? decoded.confirmed_by !== null && decoded.confirmed_at !== null : decoded.confirmed_by === null && decoded.confirmed_at === null
  if (decoded.number_of_holes !== holes.length || decoded.complete !== (decoded.holes_scored === holes.length) || !confirmationConsistent || (decoded.confirmed && !decoded.complete)) invalid('scorecard.state')
  return decoded
}

export function decodeReadScorecard(value: unknown, expectedRoundId: string, expectedOwner: ScoreOwner): ReadScorecard {
  const data = decodeObject(value, 'scorecard', 'scorekortdata')
  rejectFields(data, 'scorecard', ['confirmed_by', 'submitted_by', 'submitted_at', 'updated_at'])
  const holes = decodeArray(data.holes, 'scorecard.holes', decodeReadHole, 'scorekortdata')
  const visibility = decodeScoreVisibility(data.visibility, 'scorecard.visibility', 'scorekortdata')
  const complete = data.complete === null ? null : decodeBoolean(data.complete, 'scorecard.complete', 'scorekortdata')
  const confirmed = data.confirmed === null ? null : decodeBoolean(data.confirmed, 'scorecard.confirmed', 'scorekortdata')
  const decoded: ReadScorecard = {
    projection: 'read', round_id: decodeUuid(data.round_id, 'scorecard.round_id', 'scorekortdata'),
    owner: decodeScoreOwner(data.owner, 'scorecard.owner'), holes,
    gross_total: decodeInteger(data.gross_total, 'scorecard.gross_total', 0, undefined, 'scorekortdata'),
    net_total: decodeInteger(data.net_total, 'scorecard.net_total', undefined, undefined, 'scorekortdata'),
    playing_handicap: decodeInteger(data.playing_handicap, 'scorecard.playing_handicap', undefined, undefined, 'scorekortdata'),
    holes_scored: decodeInteger(data.holes_scored, 'scorecard.holes_scored', 0, holes.length, 'scorekortdata'),
    number_of_holes: decodeInteger(data.number_of_holes, 'scorecard.number_of_holes', 1, undefined, 'scorekortdata'),
    visible_hole_count: decodeInteger(data.visible_hole_count, 'scorecard.visible_hole_count', 1, undefined, 'scorekortdata'),
    complete, confirmed,
    confirmed_at: data.confirmed_at === null ? null : decodeTimestamp(data.confirmed_at, 'scorecard.confirmed_at', 'scorekortdata'), visibility,
  }
  validateCard(decoded.round_id, decoded.owner, expectedRoundId, expectedOwner, holes, decoded.holes_scored, decoded.gross_total, decoded.net_total, decoded.number_of_holes)
  const restricted = visibility.mode === 'front_nine'
  if (decoded.visible_hole_count !== holes.length
    || (restricted && (decoded.number_of_holes !== 18 || holes.length !== 9 || complete !== null || confirmed !== null || decoded.confirmed_at !== null))
    || (!restricted && (holes.length !== decoded.number_of_holes || complete === null || confirmed === null
      || complete !== (decoded.holes_scored === decoded.number_of_holes) || (confirmed && (!complete || decoded.confirmed_at === null))
      || (!confirmed && decoded.confirmed_at !== null)))) invalid('scorecard.visibility')
  return decoded
}
