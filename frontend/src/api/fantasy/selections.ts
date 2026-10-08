import * as d from './decode'
import { invalidData } from '../decoder'
import type { Game, Lineup, Receipt, RoundView, Source, Window } from './types'
export function game(v: unknown, t: string): Game | null { if (v === null) return null; const x = d.object(v, 'game'); return { tournament_id: d.expected(x.tournament_id, 'game.tournament_id', t), enabled: d.boolean(x.enabled, 'game.enabled'), rules_version: d.version(x.rules_version, 'game.rules_version') } }
export function lineup(v: unknown, p = 'lineup'): Lineup {
  const x = d.object(v, p), picks = d.ids(x.picks, `${p}.picks`), captain = d.uuid(x.captain, `${p}.captain`)
  if (picks.length !== 4 || !picks.includes(captain)) invalidData('Fantasy-lag', p)
  const origin = d.choice(x.origin, `${p}.origin`, ['submitted', 'carried_forward'])
  const source_round = d.nullable(x.source_round, `${p}.source_round`, d.uuid)
  if ((origin === 'carried_forward') !== (source_round !== null)) invalidData('Fantasy-lag', p)
  return { picks, captain, origin, source_round }
}
export function receipt(v: unknown, p = 'receipt'): Receipt {
  const x = d.object(v, p)
  return { ...lineup(v, p), id: d.uuid(x.id, `${p}.id`), round_id: d.uuid(x.round_id, `${p}.round_id`), user_id: d.uuid(x.user_id, `${p}.user_id`), revision: d.integer(x.revision, `${p}.revision`, 1), request_id: d.nullable(x.request_id, `${p}.request_id`, d.uuid), expected_revision: d.nullable(x.expected_revision, `${p}.expected_revision`, (a,b) => d.integer(a,b,0)), accepted_at: d.timestamp(x.accepted_at, `${p}.accepted_at`) }
}
export function window(v: unknown, r: string): Window { const x = d.object(v, 'window'); return { round_id: d.expected(x.round_id, 'window.round_id', r), deadline: d.nullable(x.deadline, 'window.deadline', d.timestamp), opened_at: d.nullable(x.opened_at, 'window.opened_at', d.timestamp), locked_at: d.nullable(x.locked_at, 'window.locked_at', d.timestamp) } }
export function roundView(v: unknown, r: string, u: string): RoundView {
  const x = d.object(v, 'round'), w = window(x.window, r)
  const selections = d.array(x.selections, 'round.selections', (v,p) => {
    const s = d.object(v,p), user_id = d.uuid(s.user_id, `${p}.user_id`), rec = d.nullable(s.receipt, `${p}.receipt`, receipt)
    if (rec && (rec.round_id !== r || rec.user_id !== user_id)) invalidData('Fantasy-kvittering', p)
    if (!w.locked_at && user_id !== u) invalidData('privat Fantasy-lag', p)
    return { user_id, state: d.choice(s.state, `${p}.state`, ['draft', 'invalid_draft', 'locked', 'missed', 'invalid', 'not_participating']), locked_at: d.nullable(s.locked_at, `${p}.locked_at`, d.timestamp), receipt: rec }
  })
  d.unique(selections.map(s => s.user_id), 'selections')
  const preview = d.nullable(x.carry_forward_preview, 'round.carry_forward_preview', receipt)
  if (preview && preview.user_id !== u) invalidData('privat Fantasy-lag', 'preview')
  return { window: w, selections, entered: d.boolean(x.entered, 'round.entered'), eligible_players: d.ids(x.eligible_players, 'round.eligible_players'), carry_forward_preview: preview, carry_forward_eligible: d.boolean(x.carry_forward_eligible, 'round.carry_forward_eligible'), selection_availability: d.choice(x.selection_availability, 'round.selection_availability', ['open', 'closed', 'not_entered', 'insufficient_players']) }
}
export function source(v: unknown, r: string, kind: string, owner: string): Source {
  const x = d.object(v, 'source'), owner_kind = d.choice(x.owner_kind, 'source.owner_kind', ['player', 'team'])
  if (owner_kind !== kind) invalidData('Fantasy-eier', 'source.owner_kind')
  return { round_id: d.expected(x.round_id, 'source.round_id', r), owner_kind, owner_id: d.expected(x.owner_id, 'source.owner_id', owner), source_token: d.token(x.source_token, 'source.source_token'), disposition_current: d.boolean(x.disposition_current, 'source.disposition_current'), disposition: d.nullable(x.disposition, 'source.disposition', (v,p) => {
    const s = d.object(v,p); return { id: d.uuid(s.id, `${p}.id`), disposed: d.boolean(s.disposed, `${p}.disposed`), correction: d.boolean(s.correction, `${p}.correction`), reason: d.string(s.reason, `${p}.reason`), actor_id: d.uuid(s.actor_id, `${p}.actor_id`), created_at: d.timestamp(s.created_at, `${p}.created_at`) }
  }) }
}
