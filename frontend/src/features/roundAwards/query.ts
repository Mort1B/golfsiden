import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from '../../api/auth'
import { awardsApi, awardsKey } from '../../api/roundAwards'
import { loadPrivateResult } from '../../api/privateResults'
export function currentAwardSession(client: QueryClient, session: AuthSession | null): boolean {
  const cached=client.getQueryData<AuthSession|null>(authKeys.session)
  return !!session && cached?.user_id===session.user_id && cached.csrf_token===session.csrf_token
}
export function awardQuery(client: QueryClient, session: AuthSession | null, tournament: string, round: string) {
  const key=awardsKey(session?.user_id??'',round)
  return queryOptions({queryKey:key,enabled:!!session,retry:false,gcTime:0,staleTime:0,
    queryFn:({signal})=>loadPrivateResult(client,{userId:session?.user_id??'',tournamentId:tournament,roundId:round},key,signal,async()=>{
      if(!currentAwardSession(client,session))throw new Error('Sesjonen er endret.')
      const view=await awardsApi.read(round,tournament,signal)
      signal.throwIfAborted()
      if(!currentAwardSession(client,session))throw new Error('Sesjonen er endret.')
      return view
    })})
}
