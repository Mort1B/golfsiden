export const awardTypes = ['closest_to_pin', 'longest_drive'] as const
export type AwardType = typeof awardTypes[number]
export const awardLabels: Record<AwardType, string> = { closest_to_pin: 'Closest to pin', longest_drive: 'Longest drive' }
export interface AwardCommand { version: 1; request_id: string; award_type: AwardType; player_id: string | null; expected_revision: string; reason: string; correction: boolean }
export interface AwardReceipt { request_id: string; round_id: string; award_type: AwardType; player_id: string | null; previous_player_id: string | null; revision: string; reason: string; correction: boolean; actor_id: string; created_at: string }
export interface AwardSlot { award_type: AwardType; player_id: string | null; revision: string }
export interface AwardsView { round_id: string; tournament_id: string; availability: 'available' | 'round_not_completed' | 'unsupported_format'; correction_required: boolean; eligible_players: { player_id: string; display_name: string }[]; awards: AwardSlot[]; history: AwardReceipt[] }
export type NetAwards = { awards: AwardType[]; adjustment: number } & ({ type: 'stroke'; base_net_total: number; adjusted_net_total: number } | { type: 'stableford'; base_net_points: number; competition_net_points: number; base_net_equivalent: number; adjusted_net_equivalent: number })
