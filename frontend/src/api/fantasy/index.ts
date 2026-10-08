import { jsonRequest, requestDecoded, requestNoContent } from '../http'
import { privateWorkspaceKeys } from '../privateWorkspace'
import { invalidData } from '../decoder'
import * as s from './selections'
import * as r from './results'
import type { Dispose, OwnerKind, Save } from './types'
export type * from './types'
export const fantasyKeys = {
  root: (u: string,t: string) => [...privateWorkspaceKeys.user(u),'tournaments',t,'fantasy'] as const,
  query: (u: string,t: string,...parts: string[]) => [...fantasyKeys.root(u,t),...parts] as const,
}
const base = (t: string) => `/api/tournaments/${encodeURIComponent(t)}/fantasy`
const round = (t: string,r: string) => `${base(t)}/rounds/${encodeURIComponent(r)}`
const owner = (t: string,r: string,k: OwnerKind,o: string) => `${round(t,r)}/owners/${k}/${encodeURIComponent(o)}`
const read = (signal?: AbortSignal): RequestInit => ({ signal,cache:'no-store' })
const write = (method:'POST'|'PUT',body: unknown,csrf: string): RequestInit => ({ ...jsonRequest(method,body,csrf),cache:'no-store' })
export const fantasyApi = {
  game: (t: string,signal?: AbortSignal) => requestDecoded(base(t),v => s.game(v,t),read(signal)),
  configure: (t: string,enabled: boolean,csrf: string) => requestDecoded(base(t),v => s.game(v,t),write('PUT',{enabled},csrf)),
  enter: (t: string,csrf: string) => requestNoContent(`${base(t)}/entry`,{ method:'POST',headers:{'x-csrf-token':csrf},cache:'no-store' }),
  round: (t: string,r: string,u: string,signal?: AbortSignal) => requestDecoded(round(t,r),v => s.roundView(v,r,u),read(signal)),
  deadline: (t: string,r: string,deadline: string|null,csrf: string) => requestDecoded(`${round(t,r)}/deadline`,v => s.window(v,r),write('PUT',{deadline},csrf)),
  save: (t: string,r: string,u: string,body: Save,csrf: string) => requestDecoded(`${round(t,r)}/lineup`,v => {
    const rec=s.receipt(v); if(rec.round_id!==r || rec.user_id!==u || rec.request_id!==body.request_id || rec.expected_revision!==body.expected_revision || rec.captain!==body.captain || rec.picks.some((p,i)=>p!==body.picks[i])) invalidData('Fantasy-kvittering','save'); return rec
  },write('PUT',body,csrf)),
  source: (t: string,r: string,k: OwnerKind,o: string,signal?: AbortSignal) => requestDecoded(owner(t,r,k,o),v=>s.source(v,r,k,o),read(signal)),
  dispose: (t: string,r: string,k: OwnerKind,o: string,body: Dispose,csrf: string) => requestDecoded(owner(t,r,k,o),v=>s.source(v,r,k,o),write('POST',body,csrf)),
  results: (t: string,signal?: AbortSignal) => requestDecoded(`${base(t)}/results`,v=>r.results(v,t),read(signal)),
  roundResults: (t: string,id: string,signal?: AbortSignal) => requestDecoded(`${round(t,id)}/results`,v=>r.roundResult(v,t,id),read(signal)),
  golfer: (t: string,id: string,signal?: AbortSignal) => requestDecoded(`${base(t)}/results/golfers/${encodeURIComponent(id)}`,v=>r.breakdown(v,t,id,r.golfer),read(signal)),
  manager: (t: string,id: string,signal?: AbortSignal) => requestDecoded(`${base(t)}/results/managers/${encodeURIComponent(id)}`,v=>r.breakdown(v,t,id,r.manager),read(signal)),
}
