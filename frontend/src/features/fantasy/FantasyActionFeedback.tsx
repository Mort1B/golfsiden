import type { useFantasyAction } from './useFantasy'
export function FantasyActionFeedback({action}:{action:ReturnType<typeof useFantasyAction>}){
  return <>{action.feedback&&<p role={action.feedback.error?'alert':'status'}>{action.feedback.text}</p>}{action.phase==='refresh_failed'&&<button type="button" disabled={action.pending} onClick={()=>void action.retryRefresh()}>Prøv oppdatering igjen</button>}</>
}
