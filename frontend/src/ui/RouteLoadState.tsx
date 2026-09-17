import { useScoringGuard } from '../features/scoring/scoringGuardContext'
import { LoadingState } from './AsyncState'

export function RouteLoading() {
  return <section className="page" aria-label="Laster side"><LoadingState /></section>
}

// Only a rejected page import renders this state. Runtime/render failures must
// not expose a reload action after unmounting a scorer and releasing its guard.
export function RouteLoadError() {
  const { blocked } = useScoringGuard()
  return <section className="page">
    <header className="page-header"><h1>Siden kunne ikke lastes</h1></header>
    <div className="state-message error" role="alert">
      <p>Kontroller nettforbindelsen og last siden på nytt for å prøve igjen.</p>
      {blocked && <p>Lagre eller forkast ulagrede scoreendringer før du laster siden på nytt.</p>}
      <button type="button" className="retry-button" disabled={blocked} onClick={() => {
        if (!blocked) window.location.reload()
      }}>Last siden på nytt</button>
    </div>
  </section>
}
