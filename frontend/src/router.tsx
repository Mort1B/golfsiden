import type { ReactNode } from 'react'
import { createBrowserRouter } from 'react-router-dom'
import { AppShell } from './ui/AppShell'
import { SignInPage } from './pages/SignInPage'
import { RequireSession } from './features/auth/RequireSession'
import { HomePage } from './pages/HomePage'
import { lazyPage } from './routing/lazyPage'

const MatchPage = lazyPage(async () => (await import('./pages/MatchPage')).MatchPage)
const MatchResultsPage = lazyPage(async () => (await import('./pages/MatchResultsPage')).MatchResultsPage)
const SharedResultsPage = lazyPage(async () => (await import('./pages/SharedResultsPage')).SharedResultsPage)
const RoundPage = lazyPage(async () => (await import('./pages/RoundPage')).RoundPage)
const TournamentPage = lazyPage(async () => (await import('./pages/TournamentPage')).TournamentPage)
const TournamentsPage = lazyPage(async () => (await import('./pages/TournamentsPage')).TournamentsPage)
const LeaderboardPage = lazyPage(async () => (await import('./pages/LeaderboardPage')).LeaderboardPage)
const ScorePage = lazyPage(async () => (await import('./pages/ScorePage')).ScorePage)
const ResetPasswordPage = lazyPage(async () => (await import('./pages/ResetPasswordPage')).ResetPasswordPage)
const TournamentOnboardingPage = lazyPage(async () => (await import('./pages/TournamentOnboardingPage')).TournamentOnboardingPage)
const JoinPage = lazyPage(async () => (await import('./pages/JoinPage')).JoinPage)
const InvitationAdminPage = lazyPage(async () => (await import('./pages/InvitationAdminPage')).InvitationAdminPage)
const TournamentManagementPage = lazyPage(async () => (await import('./pages/TournamentManagementPage')).TournamentManagementPage)
const PlayerHistoryPage = lazyPage(async () => (await import('./pages/PlayerHistoryPage')).PlayerHistoryPage)
const DirectScorecardPage = lazyPage(async () => (await import('./pages/DirectScorecardPage')).DirectScorecardPage)
const ProfilePage = lazyPage(async () => (await import('./pages/ProfilePage')).ProfilePage)

const privatePage = (children: ReactNode) => <RequireSession>{children}</RequireSession>

export const router = createBrowserRouter([
  { path: '/', element: <HomePage /> },
  { path: '/create', element: <TournamentOnboardingPage /> },
  { path: '/reset-password/:grantId', element: <ResetPasswordPage /> },
  { path: '/results/shared/:grantId', element: <SharedResultsPage /> },
  { path: '/login', element: <SignInPage /> },
  { path: '/join/:invitationId', element: <JoinPage /> },
  {
    element: <AppShell />,
    children: [
      { path: '/profile', element: privatePage(<ProfilePage />) },
      { path: '/tournaments', element: privatePage(<TournamentsPage />) },
      { path: '/tournaments/:tournamentId', element: privatePage(<TournamentPage />) },
      { path: '/tournaments/:tournamentId/invitations', element: privatePage(<InvitationAdminPage />) },
      { path: '/manage/tournaments/:tournamentId', element: privatePage(<TournamentManagementPage />) },
      { path: '/rounds/:roundId', element: privatePage(<RoundPage />) },
      { path: '/rounds/:roundId/matches', element: privatePage(<MatchPage />) },
      { path: '/rounds/:roundId/matches/:matchId', element: privatePage(<MatchPage />) },
      { path: '/rounds/:roundId/matches/:matchId/score', element: privatePage(<MatchPage scoring />) },
      { path: '/tournaments/:tournamentId/match-results', element: privatePage(<MatchResultsPage />) },
      { path: '/score', element: privatePage(<ScorePage />) },
      { path: '/leaderboard', element: privatePage(<LeaderboardPage />) },
      { path: '/tournaments/:tournamentId/results/players/:playerId', element: privatePage(<PlayerHistoryPage />) },
      { path: '/tournaments/:tournamentId/rounds/:roundId/scorecards/:ownerType/:ownerId', element: privatePage(<DirectScorecardPage />) },
    ],
  },
])
