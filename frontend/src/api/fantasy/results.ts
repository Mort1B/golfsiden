import * as d from './decode'
import { invalidData } from '../decoder'
import { isScoringFormat } from '../scoringFormats'
import { decodeScoreVisibility } from '../visibility'
import { lineup } from './selections'
import type { Breakdown, GolferResult, ManagerResult, Points, Results, RoundResult, RoundSummary, Standing } from './types'
export function points(v: unknown, p = 'points', hole = false): Points {
  const x = d.object(v,p), state = d.choice(x.state, `${p}.state`, ['not_started', 'not_participating', 'pending', 'provisional', 'settled', 'withheld', 'omitted_non_finish'])
  if (state === 'omitted_non_finish' && !hole) invalidData('Fantasy-poeng', p)
  if (state === 'pending') return { state, recorded: d.integer(x.recorded, `${p}.recorded`) }
  if (state === 'provisional' || state === 'settled') return { state, total: d.integer(x.total, `${p}.total`) }
  if ('total' in x || 'recorded' in x) invalidData('Fantasy-poeng', p)
  return { state }
}
function summary(v: unknown, p: string): RoundSummary {
  const x = d.object(v,p)
  if (!isScoringFormat(x.format)) invalidData('Fantasy-format', `${p}.format`)
  return { round_id: d.uuid(x.round_id, `${p}.round_id`), round_number: d.integer(x.round_number, `${p}.round_number`,1), name: d.string(x.name, `${p}.name`), format: x.format, sporting_status: d.nullable(x.sporting_status, `${p}.sporting_status`, (v,p) => d.choice(v,p,['draft','open','completed','locked'])), visibility: decodeScoreVisibility(x.visibility, `${p}.visibility`, 'Fantasy'), points: points(x.points, `${p}.points`) }
}
function standing(v: unknown, p: string): Standing {
  const x = d.object(v,p), result = { id: d.uuid(x.id, `${p}.id`), display_name: d.string(x.display_name, `${p}.display_name`), points: points(x.points, `${p}.points`), rank: d.rank(x.rank, `${p}.rank`), rounds: d.array(x.rounds, `${p}.rounds`, (v,p) => { const r = d.object(v,p); return { round_id: d.uuid(r.round_id, `${p}.round_id`), points: points(r.points, `${p}.points`) } }) }
  validateRank(result.points, result.rank, p); d.unique(result.rounds.map(r => r.round_id), p)
  return result
}
function validateRank(p: Points, rank: number | null, path: string): void {
  if (rank !== null && p.state !== 'settled' && p.state !== 'provisional') invalidData('Fantasy-plassering', path)
}
export function golfer(v: unknown, p: string): GolferResult {
  const x = d.object(v,p)
  const result = { player_id: d.uuid(x.player_id, `${p}.player_id`), display_name: d.string(x.display_name, `${p}.display_name`), points: points(x.points, `${p}.points`), rank: d.rank(x.rank, `${p}.rank`), team_id: d.nullable(x.team_id, `${p}.team_id`, d.uuid), recorded_hole_points: d.nullable(x.recorded_hole_points, `${p}.recorded_hole_points`, d.integer), placement_points: d.nullable(x.placement_points, `${p}.placement_points`, d.integer), settlement: d.nullable(x.settlement, `${p}.settlement`, (v,p) => d.choice(v,p,['playing','unconfirmed','confirmed','non_finish','stale_non_finish','withheld'])), match_outcome: d.nullable(x.match_outcome, `${p}.match_outcome`, (v,p) => d.choice(v,p,['win','draw','loss'])), holes: d.array(x.holes, `${p}.holes`, (v,p) => {
    const h = d.object(v,p); return { hole_id: d.uuid(h.hole_id, `${p}.hole_id`), hole_number: d.integer(h.hole_number, `${p}.hole_number`,1,18), par: d.integer(h.par, `${p}.par`,1), net_strokes: d.nullable(h.net_strokes, `${p}.net_strokes`, d.integer), category: d.nullable(h.category, `${p}.category`, (v,p) => d.choice(v,p,['ace','albatross_or_better','eagle','birdie','par','bogey','double_bogey','triple_bogey','quadruple_or_worse','pickup'])), points: points(h.points, `${p}.points`,true) }
  }) }
  validateRank(result.points, result.rank, p); d.unique(result.holes.map(h => h.hole_id), p)
  return result
}
export function manager(v: unknown, p: string): ManagerResult {
  const x = d.object(v,p)
  const result = { user_id: d.uuid(x.user_id, `${p}.user_id`), display_name: d.string(x.display_name, `${p}.display_name`), points: points(x.points, `${p}.points`), rank: d.rank(x.rank, `${p}.rank`), selection_state: d.choice(x.selection_state, `${p}.selection_state`, ['unlocked','locked','missed','invalid','not_participating','pending']), lineup: d.nullable(x.lineup, `${p}.lineup`, lineup), contributions: d.array(x.contributions, `${p}.contributions`, (v,p) => {
    const c = d.object(v,p), captain = d.boolean(c.captain, `${p}.captain`), multiplier = d.integer(c.multiplier, `${p}.multiplier`,1,2)
    if (multiplier !== (captain ? 2 : 1)) invalidData('kapteinfaktor', p)
    return { player_id: d.uuid(c.player_id, `${p}.player_id`), captain, multiplier, base_points: points(c.base_points, `${p}.base_points`), points: points(c.points, `${p}.points`) }
  }) }
  validateRank(result.points, result.rank, p); d.unique(result.contributions.map(c => c.player_id), p)
  if (!result.lineup && result.contributions.length) invalidData('privat Fantasy-lag', p)
  if (result.lineup && result.contributions.some(c => !result.lineup?.picks.includes(c.player_id) || c.captain !== (c.player_id === result.lineup.captain))) invalidData('Fantasy-lag',p)
  return result
}
function envelope(v: unknown, t: string) { const x = d.object(v,'results'); return { x, tournament_id: d.expected(x.tournament_id,'results.tournament_id',t), revision: d.token(x.revision,'results.revision') } }
function sameRounds(ids: string[], expected: string[], p: string): void {
  if (ids.length !== expected.length || ids.some((id,i)=>id!==expected[i])) invalidData('Fantasy-runder',p)
}
function visibleRound(round: RoundSummary, golfers: GolferResult[], managers: ManagerResult[]): void {
  if(round.visibility.mode!=='front_nine')return
  if(round.sporting_status!==null || !['withheld','not_started'].includes(round.points.state)) invalidData('skjult Fantasy-runde','round')
  for(const g of golfers){
    if(!['withheld','not_started','not_participating'].includes(g.points.state)||g.rank!==null||g.placement_points!==null||g.match_outcome!==null||g.settlement!==null&&g.settlement!=='withheld')invalidData('skjulte Fantasy-poeng','golfer')
    for(const h of g.holes)if(h.hole_number>9&&(h.points.state!=='withheld'||h.category!==null||h.net_strokes!==null))invalidData('skjult Fantasy-hull','hole')
    if(round.format==='singles_match_play'&&(g.holes.length>0||g.recorded_hole_points!==null))invalidData('skjult Fantasy-match','golfer')
  }
  for(const m of managers)if(m.contributions.some(c=>!['withheld','not_started','not_participating'].includes(c.points.state)||!['withheld','not_started','not_participating'].includes(c.base_points.state)))invalidData('skjult Fantasy-bidrag','manager')
}
export function results(v: unknown, t: string): Results {
  const { x, ...base } = envelope(v,t)
  const result = { ...base, rules_version: d.version(x.rules_version,'results.rules_version'), rounds: d.array(x.rounds,'results.rounds',summary), golfers: d.array(x.golfers,'results.golfers',standing), managers: d.array(x.managers,'results.managers',standing) }
  d.unique(result.rounds.map(r => r.round_id),'results.rounds'); d.unique(result.golfers.map(r => r.id),'results.golfers'); d.unique(result.managers.map(r => r.id),'results.managers')
  for(const board of [result.golfers,result.managers])for(const s of board)sameRounds(s.rounds.map(r=>r.round_id),result.rounds.map(r=>r.round_id),'results.standing.rounds')
  for(const r of result.rounds)visibleRound(r,[],[])
  return result
}
export function roundResult(v: unknown, t: string, r: string): RoundResult {
  const { x, ...base } = envelope(v,t), round = summary(x.round,'results.round')
  if (round.round_id !== r) invalidData('Fantasy-runde','results.round')
  const golfers = d.array(x.golfers,'results.golfers',golfer), managers = d.array(x.managers,'results.managers',manager)
  d.unique(golfers.map(g => g.player_id),'results.golfers'); d.unique(managers.map(m => m.user_id),'results.managers')
  visibleRound(round,golfers,managers)
  return { ...base, round, golfers, managers }
}
export function breakdown<T extends GolferResult | ManagerResult>(v: unknown,t: string,id: string,decode: (v: unknown,p: string) => T): Breakdown<T> {
  const { x, ...base } = envelope(v,t), s = standing(x.standing,'breakdown.standing')
  if (s.id !== id) invalidData('Fantasy-deltaker','breakdown.standing')
  const rounds = d.array(x.rounds,'breakdown.rounds',(v,p) => { const r = d.object(v,p), result = decode(r.result,`${p}.result`); if (('player_id' in result ? result.player_id : result.user_id) !== id) invalidData('Fantasy-deltaker',p); return { round: summary(r.round,`${p}.round`), result } })
  d.unique(rounds.map(r => r.round.round_id),'breakdown.rounds')
  sameRounds(s.rounds.map(r=>r.round_id),rounds.map(r=>r.round.round_id),'breakdown.rounds')
  for(const row of rounds)visibleRound(row.round,'player_id' in row.result?[row.result]:[],'user_id' in row.result?[row.result]:[])
  return { ...base, standing:s, rounds }
}
