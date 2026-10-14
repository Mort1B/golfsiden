import { decodeArray, decodeInteger, decodeObject } from '../decoder'
import { decodeAwardType } from '../roundAwards/decode'
import type { NetAwards } from '../roundAwards/types'
import type { StablefordResult } from '../types'
import { invalidLeaderboard } from './shared'
export function decodeNetAwards(value: unknown, path: string): NetAwards {
  const d=decodeObject(value,path)
  const awards=decodeArray(d.awards,path,decodeAwardType)
  const adjustment=decodeInteger(d.adjustment,path,-2,-1)
  if(awards.length!==-adjustment || new Set(awards).size!==awards.length) invalidLeaderboard(path)
  if(d.type==='stroke') {
    const base_net_total=decodeInteger(d.base_net_total,path), adjusted_net_total=decodeInteger(d.adjusted_net_total,path)
    if(adjusted_net_total!==base_net_total+adjustment) invalidLeaderboard(path)
    return {type:'stroke',awards,adjustment,base_net_total,adjusted_net_total}
  }
  if(d.type!=='stableford') invalidLeaderboard(path)
  const base_net_points=decodeInteger(d.base_net_points,path,0), competition_net_points=decodeInteger(d.competition_net_points,path,0), base_net_equivalent=decodeInteger(d.base_net_equivalent,path), adjusted_net_equivalent=decodeInteger(d.adjusted_net_equivalent,path)
  if(competition_net_points!==base_net_points-adjustment || adjusted_net_equivalent!==base_net_equivalent+adjustment) invalidLeaderboard(path)
  return {type:'stableford',awards,adjustment,base_net_points,competition_net_points,base_net_equivalent,adjusted_net_equivalent}
}
export function validateNetAwards(award: NetAwards | undefined, value: StablefordResult | undefined, net: number | undefined, permitted: boolean, path: string, teamBase = false): void {
  if(!award)return
  if(!permitted || (award.type==='stableford') !== !!value) invalidLeaderboard(path)
  if(award.type==='stroke' && (teamBase?award.base_net_total:award.adjusted_net_total)!==net) invalidLeaderboard(path)
  if(award.type==='stableford' && (award.base_net_points!==value?.net_points || award.base_net_equivalent!==value.net_equivalent)) invalidLeaderboard(path)
}
