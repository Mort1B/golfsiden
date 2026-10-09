import { describe, expect, it } from 'vitest'
import { fourBallFixture } from '../../api/fourBall/fixtures'
import { stablefordFixture } from '../../api/stableford/fixtures'
import { enqueueFourBall, enqueueStableford, type FourBallTarget, type StablefordTarget } from './offline/model'
import { cardProgress } from './cardProgress'
const account='account',tournament='tournament'
const absent={type:'absent'} as const
function sideTarget(slot:0|1,hole=0):FourBallTarget {const card=fourBallFixture();return {protocol:'four_ball_v1',accountId:account,tournamentId:tournament,roundId:card.round_id,sideId:card.owner.id,owner:{type:'player',id:card.partners[slot].player_id},holeId:card.holes[hole]?.hole_id??'',holeNumber:hole+1}}
function singleTarget(hole=0):StablefordTarget {const card=stablefordFixture();return {protocol:'stableford_v1',accountId:account,tournamentId:tournament,roundId:card.round_id,owner:card.owner,holeId:card.holes[hole]?.hole_id??'',holeNumber:hole+1}}
describe('display-only distinct-hole progress',()=>{
 it('counts two partner edits and repeated corrections on one hole once',()=>{
  const a=enqueueFourBall(null,sideTarget(0),{type:'numeric',gross_strokes:4},absent),b=enqueueFourBall(null,sideTarget(1),{type:'numeric',gross_strokes:5},absent)
  const correction=enqueueFourBall(a,sideTarget(0),{type:'numeric',gross_strokes:6},absent)
  expect(cardProgress(fourBallFixture(),account,tournament,[a,b,correction],[])).toEqual({entered:1,verified:0,pending:1,total:18})
 })
 it('does not count a double pickup as a resolved four-ball hole',()=>{
  const items=([0,1] as const).map(slot=>enqueueFourBall(null,sideTarget(slot),{type:'no_score'},absent))
  expect(cardProgress(fourBallFixture(),account,tournament,items,[])).toEqual({entered:0,verified:0,pending:1,total:18})
 })
 it('does count Stableford pickup and keeps server totals unchanged',()=>{
  const card=stablefordFixture(),before=JSON.stringify(card),item=enqueueStableford(null,singleTarget(),{type:'no_score'},absent)
  expect(cardProgress(card,account,tournament,[item],[])).toEqual({entered:1,verified:0,pending:1,total:18})
  expect(JSON.stringify(card)).toBe(before)
 })
 it('excludes a pending correction from verified current holes without inflating entered progress',()=>{
  const item=enqueueStableford(null,singleTarget(),{type:'numeric',gross_strokes:7},absent)
  expect(cardProgress(stablefordFixture('numeric'),account,tournament,[item],[])).toEqual({entered:18,verified:17,pending:1,total:18})
 })
 it('ignores other accounts, tournaments, rounds, side and protocol',()=>{
  const target=sideTarget(0)
  const items=[{...target,accountId:'other'},{...target,tournamentId:'other'},{...target,roundId:'other'},{...target,sideId:'other'}].map(t=>enqueueFourBall(null,t,{type:'numeric',gross_strokes:4},absent))
  expect(cardProgress(fourBallFixture(),account,tournament,items,[])).toEqual({entered:0,verified:0,pending:0,total:18})
  expect(cardProgress(stablefordFixture(),account,tournament,items,[]).entered).toBe(0)
 })
 it('lets nondurable current intent override an older queued input without claiming device storage',()=>{
  const target=sideTarget(0),item=enqueueFourBall(null,target,{type:'numeric',gross_strokes:4},absent)
  expect(cardProgress(fourBallFixture(),account,tournament,[item],[{kind:'four_ball',slot:1,target,value:{type:'no_score'},key:item.key,expected:absent,sequence:1,predecessor:null,saving:false,error:'storage failed',recovery:false}])).toEqual({entered:0,verified:0,pending:1,total:18})
 })
})
