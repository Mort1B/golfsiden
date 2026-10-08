import type { RoundStatus, ScoringFormat } from '../types'
import type { ScoreVisibility } from '../visibility'
export interface Game { tournament_id: string; enabled: boolean; rules_version: number }
export interface Window { round_id: string; deadline: string | null; opened_at: string | null; locked_at: string | null }
export interface Lineup { picks: string[]; captain: string; origin: 'submitted' | 'carried_forward'; source_round: string | null }
export interface Receipt extends Lineup { id: string; round_id: string; user_id: string; revision: number; request_id: string | null; expected_revision: number | null; accepted_at: string }
export type SelectionState = 'draft' | 'invalid_draft' | 'locked' | 'missed' | 'invalid' | 'not_participating'
export interface Selection { user_id: string; state: SelectionState; locked_at: string | null; receipt: Receipt | null }
export interface RoundView { window: Window; entered: boolean; eligible_players: string[]; selections: Selection[]; carry_forward_preview: Receipt | null; carry_forward_eligible: boolean; selection_availability: 'open' | 'closed' | 'not_entered' | 'insufficient_players' }
export interface Save { request_id: string; expected_revision: number; picks: string[]; captain: string }
export type OwnerKind = 'player' | 'team'
export interface Disposition { id: string; disposed: boolean; correction: boolean; reason: string; actor_id: string; created_at: string }
export interface Source { round_id: string; owner_kind: OwnerKind; owner_id: string; source_token: string; disposition: Disposition | null; disposition_current: boolean }
export interface Dispose { expected_source_token: string; disposed: boolean; correction: boolean; reason: string }
export type Points = { state: 'not_started' | 'not_participating' | 'withheld' | 'omitted_non_finish' } | { state: 'pending'; recorded: number } | { state: 'settled' | 'provisional'; total: number }
export interface RoundSummary { round_id: string; round_number: number; name: string; format: ScoringFormat; sporting_status: RoundStatus | null; visibility: ScoreVisibility; points: Points }
export interface Standing { id: string; display_name: string; points: Points; rank: number | null; rounds: { round_id: string; points: Points }[] }
export interface Results { tournament_id: string; rules_version: number; revision: string; rounds: RoundSummary[]; golfers: Standing[]; managers: Standing[] }
export type Category = 'ace' | 'albatross_or_better' | 'eagle' | 'birdie' | 'par' | 'bogey' | 'double_bogey' | 'triple_bogey' | 'quadruple_or_worse' | 'pickup'
export interface HoleResult { hole_id: string; hole_number: number; par: number; category: Category | null; net_strokes: number | null; points: Points }
export type Settlement = 'playing' | 'unconfirmed' | 'confirmed' | 'non_finish' | 'stale_non_finish' | 'withheld'
export interface GolferResult { player_id: string; display_name: string; points: Points; rank: number | null; team_id: string | null; holes: HoleResult[]; recorded_hole_points: number | null; placement_points: number | null; settlement: Settlement | null; match_outcome: 'win' | 'draw' | 'loss' | null }
export interface PickResult { player_id: string; captain: boolean; multiplier: number; base_points: Points; points: Points }
export interface ManagerResult { user_id: string; display_name: string; points: Points; rank: number | null; selection_state: 'unlocked' | 'locked' | 'missed' | 'invalid' | 'not_participating' | 'pending'; lineup: Lineup | null; contributions: PickResult[] }
export interface RoundResult { tournament_id: string; revision: string; round: RoundSummary; golfers: GolferResult[]; managers: ManagerResult[] }
export interface Breakdown<T> { tournament_id: string; revision: string; standing: Standing; rounds: { round: RoundSummary; result: T }[] }
