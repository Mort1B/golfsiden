import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { RouterProvider } from 'react-router-dom'
import { router } from './router'
import { AuthProvider } from './features/auth/AuthProvider'
import { ScoringGuardProvider } from './features/scoring/ScoringGuardProvider'
import './styles.css'
import './features/leaderboards/leaderboards.css'
import './features/scoring/scoring.css'
import './features/auth/auth.css'
import './features/onboarding/onboarding.css'
import './features/onboarding/onboarding-details.css'
import './features/onboarding/onboarding-success.css'
import './features/onboarding/home.css'
import './features/invitations/invitations.css'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 20_000, retry: 1 },
  },
})

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <AuthProvider>
        <ScoringGuardProvider>
          <RouterProvider router={router} />
        </ScoringGuardProvider>
      </AuthProvider>
    </QueryClientProvider>
  </StrictMode>,
)
