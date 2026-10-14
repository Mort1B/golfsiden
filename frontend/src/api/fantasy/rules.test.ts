import { expect, it } from 'vitest'
import { game } from './selections'
import { breakdown, golfer, manager, results, roundResult } from './results'
import { id, tournament, roundId, user } from '../../features/fantasy/fixtures'
const summary={round_id:roundId,round_number:1,name:'Round',format:'individual_stroke_play',sporting_status:'completed',visibility:{mode:'full'},points:{state:'settled',total:20}}
const standing={id:user,display_name:'Manager',points:{state:'settled',total:20},rank:1,rounds:[{round_id:roundId,points:{state:'settled',total:20}}]}
const g={player_id:user,display_name:'Player',points:{state:'settled',total:20},rank:1,team_id:null,holes:[],recorded_hole_points:20,placement_points:0,settlement:'confirmed',match_outcome:null}
const m={user_id:user,display_name:'Manager',points:{state:'settled',total:20},rank:1,selection_state:'missed',lineup:null,contributions:[]}
const base={tournament_id:tournament,revision:'a'.repeat(43)}
it.each([1,2])('requires a supported version on every envelope (%s)',version=>{
 const payload={...base,rules_version:version}
 const decoders=[(rules:unknown)=>game({...payload,enabled:true,rules_version:rules},tournament), (rules:unknown)=>results({...payload,rules_version:rules,rounds:[summary],golfers:[standing],managers:[]},tournament), (rules:unknown)=>roundResult({...payload,rules_version:rules,round:summary,golfers:[g],managers:[m]},tournament,roundId), (rules:unknown)=>breakdown({...payload,rules_version:rules,standing,rounds:[{round:summary,result:g}]},tournament,user,golfer), (rules:unknown)=>breakdown({...payload,rules_version:rules,standing,rounds:[{round:summary,result:m}]},tournament,user,manager)]
 for(const decode of decoders){expect(decode(version)?.rules_version).toBe(version);for(const invalid of [undefined,null,0,3,'2',1.5])expect(()=>decode(invalid)).toThrow()}
})
it.each([{version:1 as const,ace:10,albatross:10},{version:2 as const,ace:20,albatross:5}])('validates category awards under version $version without changing server totals',({version,ace,albatross})=>{
 for(const [category,total] of [['ace',ace],['albatross_or_better',albatross],['eagle',3],['pickup',-5]] as const){
  const hole={hole_id:id(40),hole_number:1,par:4,category,net_strokes:category==='pickup'?null:1,points:{state:'settled',total}}
  const payload={...base,rules_version:version,round:summary,golfers:[{...g,holes:[hole]}],managers:[]}
  expect(roundResult(payload,tournament,roundId).golfers[0]?.holes[0]?.points).toEqual(hole.points)
  expect(()=>roundResult({...payload,golfers:[{...g,holes:[{...hole,points:{state:'settled',total:total+1}}]}]},tournament,roundId)).toThrow()
  expect(()=>breakdown({...base,rules_version:version,standing,rounds:[{round:summary,result:{...g,holes:[{...hole,points:{state:'settled',total:total+1}}]}}]},tournament,user,golfer)).toThrow()
 }
})
it('keeps missing and concealed holes distinct from scored categories',()=>{
 for(const state of ['pending','withheld','omitted_non_finish']){
 const hole={hole_id:id(40),hole_number:1,par:4,category:null,net_strokes:null,points:state==='pending'?{state,recorded:0}:{state}}
 expect(golfer({...g,holes:[hole]},'golfer',2).holes[0]?.category).toBeNull()
 expect(()=>golfer({...g,holes:[{...hole,category:'ace'}]},'golfer',2)).toThrow()
 }
})
