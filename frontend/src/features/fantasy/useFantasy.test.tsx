// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { authKeys } from '../../api/auth'
import { ApiHttpError } from '../../api/http'
import { AuthContext } from '../auth/authContext'
import { useFantasyQuery } from './useFantasy'
import { session, tournament } from './fixtures'
let client:QueryClient
beforeEach(()=>{client=new QueryClient();client.setQueryData(authKeys.session,session)})
afterEach(()=>{cleanup();client.clear()})
function Projection({load}:{load:(signal:AbortSignal)=>Promise<string>}){const query=useFantasyQuery(tournament,['results'],load);return query.error?<p role="alert">Kunne ikke laste</p>:<p>{query.data??'Laster'}</p>}
function tree(load:(signal:AbortSignal)=>Promise<string>){return render(<QueryClientProvider client={client}><AuthContext.Provider value={{session,loading:false,error:null,signIn:vi.fn(),establishSession:vi.fn(),signOut:vi.fn(),retry:vi.fn()}}><Projection load={load}/></AuthContext.Provider></QueryClientProvider>)}
it('retries only transient authoritative read conflicts up to twice',async()=>{const load=vi.fn().mockRejectedValueOnce(new ApiHttpError(409,'fantasy_conflict','conflict')).mockRejectedValueOnce(new ApiHttpError(409,'fantasy_conflict','conflict')).mockResolvedValue('Oppdatert');tree(load);await screen.findByText('Oppdatert');expect(load).toHaveBeenCalledTimes(3)})
it.each([[403,'forbidden'],[409,'fantasy_closed'],[503,'unavailable']] as const)('does not retry a %s %s response',async(status,code)=>{const load=vi.fn().mockRejectedValue(new ApiHttpError(status,code,'error'));tree(load);await screen.findByRole('alert');expect(load).toHaveBeenCalledOnce()})
