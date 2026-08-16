import { createBrowserRouter } from 'react-router-dom'
import { AppShell } from './ui/AppShell'
import { PlayersPage } from './pages/PlayersPage'
import { RoundPage } from './pages/RoundPage'
import { TournamentPage } from './pages/TournamentPage'
import { TournamentsPage } from './pages/TournamentsPage'
import { LeaderboardPage } from './pages/LeaderboardPage'
import { ScorePage } from './pages/ScorePage'
import { SignInPage } from './pages/SignInPage'
import { RequireSession } from './features/auth/RequireSession'
import { HomePage } from './pages/HomePage'
import { TournamentOnboardingPage } from './pages/TournamentOnboardingPage'
import { JoinPage } from './pages/JoinPage'
import { InvitationAdminPage } from './pages/InvitationAdminPage'

export const router = createBrowserRouter([
  { path: '/', element: <HomePage /> },
  { path: '/create', element: <TournamentOnboardingPage /> },
  { path: '/login', element: <SignInPage /> },
  { path: '/join/:invitationId', element: <JoinPage /> },
  {
    element: <AppShell />,
    children: [
      { path: '/tournaments', element: <RequireSession><TournamentsPage /></RequireSession> },
      { path: '/tournaments/:tournamentId', element: <TournamentPage /> },
      { path: '/tournaments/:tournamentId/invitations', element: <RequireSession><InvitationAdminPage /></RequireSession> },
      { path: '/rounds/:roundId', element: <RoundPage /> },
      { path: '/players', element: <PlayersPage /> },
      { path: '/score', element: <RequireSession><ScorePage /></RequireSession> },
      { path: '/leaderboard', element: <LeaderboardPage /> },
    ],
  },
])
