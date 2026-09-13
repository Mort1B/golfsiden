import { useEffect, useReducer } from 'react'
import { useLocation, useParams } from 'react-router-dom'
import { ResetPasswordExperience } from '../features/recovery/ResetPasswordExperience'

export function ResetPasswordPage() {
  const { grantId = '' } = useParams()
  const location = useLocation()
  const [hashVisit, revisitHash] = useReducer((value: number) => value + 1, 0)
  useEffect(() => {
    // Reopening the same fragment is a fresh visit even if the router retained
    // its prior location after the secret was removed with replaceState.
    const onHashChange = () => { if (window.location.hash) revisitHash() }
    window.addEventListener('hashchange', onHashChange)
    return () => window.removeEventListener('hashchange', onHashChange)
  }, [])
  return <main className="sign-in-page recovery-page">
    <ResetPasswordExperience key={`${grantId}:${location.key}:${hashVisit}`} grantId={grantId} fragment={location.hash} />
  </main>
}
