import { cleanup, fireEvent, render, screen, waitFor, act } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, expect, it, vi } from 'vitest'
import { StablefordSettings } from '/home/morten/Prog/guttasgolfside/golfsiden/frontend/src/features/tournaments/StablefordSettings'
import { round as draft, session } from '/home/morten/Prog/guttasgolfside/golfsiden/frontend/src/features/tournaments/lifecycle/__tests__/fixtures'
import { AuthContext } from '/home/morten/Prog/guttasgolfside/golfsiden/frontend/src/features/auth/authContext'
import { publishSessionTransition } from '/home/morten/Prog/guttasgolfside/golfsiden/frontend/src/features/auth/sessionTransition'
import { authKeys } from '/home/morten/Prog/guttasgolfside/golfsiden/frontend/src/api/auth'
import { tournamentKeys } from '/home/morten/Prog/guttasgolfside/golfsiden/frontend/src/api/tournaments'
import { stablefordApi } from '/home/morten/Prog/guttasgolfside/golfsiden/frontend/src/api/stableford'
const round={...draft,scoring_format:'individual_stableford' as const}
afterEach(()=>{cleanup();vi.restoreAllMocks()})
for(const transition of ['logout','account-switch','control'] as const) it(`PERSIST-3 delayed settings response after ${transition}`,async()=>{
 const client=new QueryClient({defaultOptions:{queries:{retry:false},mutations:{retry:false}}})
 client.setQueryData(authKeys.session,session)
 const key=tournamentKeys.round(session.user_id,round.id)
 client.setQueryData(key,round)
 let finish:(value:typeof round)=>void=()=>undefined
 const held=new Promise<typeof round>(resolve=>{finish=resolve})
 const save=vi.spyOn(stablefordApi,'settings').mockReturnValue(held)
 const auth={session,loading:false,error:null,signIn:vi.fn(),signOut:vi.fn(),establishSession:vi.fn(),retry:vi.fn()}
 const mounted=render(<QueryClientProvider client={client}><AuthContext value={auth}><StablefordSettings round={round}/></AuthContext></QueryClientProvider>)
 fireEvent.change(screen.getByLabelText('Handicapandel (0–100 %)'),{target:{value:'50'}})
 fireEvent.click(screen.getByRole('button'))
 await waitFor(()=>expect(save).toHaveBeenCalledOnce())
 if(transition!=='control') {
  act(()=>publishSessionTransition(client,transition==='logout'?null:{...session,user_id:'00000000-0000-0000-0000-000000000099',csrf_token:'synthetic-new-session'}))
  mounted.unmount()
  expect(client.getQueriesData({queryKey:['private-workspace']})).toHaveLength(0)
 }
 await act(async()=>finish({...round,handicap_allowance_percent:50}))
 await waitFor(()=>expect(client.getQueryData<typeof round>(key)?.handicap_allowance_percent).toBe(50))
 if(transition==='logout') expect(client.getQueryData(authKeys.session)).toBeNull()
 if(transition==='account-switch') expect(client.getQueryData<typeof session>(authKeys.session)?.user_id).toBe('00000000-0000-0000-0000-000000000099')
 console.log(`PERSIST-3 ${transition}: delayed success writes captured-account round cache`)
 client.clear()
})
