import { Navigate } from 'react-router-dom'
import { useState, type ReactNode } from 'react'
import { useAuth } from '../features/auth/authContext'
import { OnboardingWizard } from '../features/onboarding/OnboardingWizard'
import { ErrorState, LoadingState } from '../ui/AsyncState'

export function TournamentOnboardingPage() {
  const auth = useAuth()
  const [createdHere, setCreatedHere] = useState(false)
  if (auth.loading) return <OnboardingState><LoadingState /></OnboardingState>
  if (auth.error) return <OnboardingState><ErrorState error={auth.error} onRetry={() => void auth.retry()} /></OnboardingState>
  if (auth.session && !createdHere) return <Navigate replace to="/tournaments" />
  return <OnboardingWizard onCreated={() => setCreatedHere(true)} />
}

function OnboardingState({ children }: { children: ReactNode }) {
  return <main className="onboarding-page"><section className="onboarding-shell"><p className="brand">Guttas Golf</p>{children}</section></main>
}
