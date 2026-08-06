import { createBrowserRouter } from 'react-router-dom'
import { AppShell } from './ui/AppShell'
import { PlaceholderPage } from './ui/PlaceholderPage'
import { PlayersPage } from './pages/PlayersPage'
import { RoundPage } from './pages/RoundPage'
import { TournamentPage } from './pages/TournamentPage'
import { TournamentsPage } from './pages/TournamentsPage'
import { LeaderboardPage } from './pages/LeaderboardPage'
import { ScorePage } from './pages/ScorePage'
import { SignInPage } from './pages/SignInPage'
import { RequireSession } from './features/auth/RequireSession'

export const router = createBrowserRouter([
  { path: '/login', element: <SignInPage /> },
  {
    element: <AppShell />,
    children: [
      { path: '/', element: <TournamentsPage /> },
      { path: '/tournaments/:tournamentId', element: <TournamentPage /> },
      { path: '/rounds/:roundId', element: <RoundPage /> },
      { path: '/players', element: <PlayersPage /> },
      { path: '/score', element: <RequireSession><ScorePage /></RequireSession> },
      { path: '/leaderboard', element: <LeaderboardPage /> },
      { path: '/admin', element: <RequireSession role="admin"><PlaceholderPage title="Admin" state="Velg en turnering" /></RequireSession> },
    ],
  },
])
