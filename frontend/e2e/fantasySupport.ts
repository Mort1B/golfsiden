import { expect, type Page } from '@playwright/test'
import { decodeAuthSession } from '../src/api/auth'
import { decodeObject, decodeUuid } from '../src/api/decoder'
import { decodeTournament, decodeRound, decodeTournamentRounds } from '../src/api/tournaments/decoders'
export async function fantasyFixture(page:Page){
  const stamp=`${Date.now()}_${Math.floor(Math.random()*10000)}`,day=new Date(Date.now()+86400000).toISOString().slice(0,10)
  const username=`fantasy_${stamp}`,password='fantasy-local-browser-fixture'
  const response=await page.request.post('/api/onboarding/tournaments',{data:{creator:{account:{username,password},player:{display_name:'Arrangør med langt navn for Fantasy',handicap_index:12}},tournament:{name:'Fantasy med svært langt turneringsnavn for mobilvisning',description:'',start_date:day,end_date:day,counted_rounds:1,mandatory_round_number:null},rounds:[{round_number:1,name:'Første runde med langt navn',round_date:day,scoring_format:'individual_stroke_play'},{round_number:2,name:'Andre runde',round_date:day,scoring_format:'individual_stroke_play'}]}})
  expect(response.status()).toBe(201)
  const body=decodeObject(await response.json(),'onboarding'),tournament=decodeTournament(body.tournament),auth=decodeAuthSession(body.session)
  if(!auth.player_id)throw new Error('Missing fixture player')
  const names=[auth.display_name],players=[auth.player_id]
  const mutate=async(path:string,data:unknown={},method='POST')=>{const response=await page.request.fetch(path,{method,data,headers:{'x-csrf-token':auth.csrf_token}});expect(response.ok(),`${path}: ${response.status()}`).toBe(true);return response}
  for(let i=1;i<=3;i++){const name=`Fantasyspiller ${i} med et svært langt navn som skal brytes på mobil`,created=decodeObject(await(await mutate(`/api/tournaments/${tournament.id}/players`,{display_name:name,handicap_index:10+i})).json(),'player');players.push(decodeUuid(created.player_id,'player.player_id'));names.push(name)}
  const rounds=decodeTournamentRounds(await(await page.request.get(`/api/tournaments/${tournament.id}/rounds`)).json(),tournament.id)
  async function prepare(){for(const round of rounds){const configured=decodeRound(await(await mutate(`/api/rounds/${round.id}/course-configuration`,{expected_round_updated_at:round.updated_at,selection:{source:'manual',course_name:'Fantasy testbane',location:null,tee:{category:'male',name:'Gul',course_rating:72,slope_rating:113,holes:Array.from({length:18},(_,i)=>({par:4,stroke_index:i+1,distance:null}))}}},'PUT')).json());await mutate(`/api/rounds/${round.id}/pairings`,{expected_round_updated_at:configured.updated_at,teams:[],flights:[{id:crypto.randomUUID(),name:'Fantasyflight',starting_hole:1,tee_time:null,members:players.map(player_id=>({player_id}))}],legacy_conversions:[]},'PUT')}
    const fresh=decodeTournament(await(await page.request.get(`/api/tournaments/${tournament.id}`)).json());await mutate(`/api/tournaments/${tournament.id}/start`,{expected_tournament_updated_at:fresh.updated_at})
  }
  return {tournament,auth,players,names,rounds,mutate,prepare,username,password,url:`/tournaments/${tournament.id}/fantasy`}
}
export async function fantasyLayouts(page:Page,name:string){for(const width of [320,390,1280]){await page.setViewportSize({width,height:900});expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);for(const control of await page.locator('.fantasy-page button:visible,.fantasy-page select:visible').all()){const box=await control.boundingBox();expect(box?.height).toBeGreaterThanOrEqual(44)}await page.screenshot({path:`/tmp/golf-fantasy-${name}-${width}.png`,fullPage:true})}}
