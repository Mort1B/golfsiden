import { createBrowserRouter } from 'react-router-dom'
import { AppShell } from './ui/AppShell'
import { PlaceholderPage } from './ui/PlaceholderPage'
import { PlayersPage } from './pages/PlayersPage'
import { RoundPage } from './pages/RoundPage'
import { TournamentPage } from './pages/TournamentPage'
import { TournamentsPage } from './pages/TournamentsPage'
import { LeaderboardPage } from './pages/LeaderboardPage'
import { ScorePage } from './pages/ScorePage'

export const router = createBrowserRouter([
  {
    element: <AppShell />,
    children: [
      { path: '/', element: <TournamentsPage /> },
      { path: '/tournaments/:tournamentId', element: <TournamentPage /> },
      { path: '/rounds/:roundId', element: <RoundPage /> },
      { path: '/players', element: <PlayersPage /> },
      { path: '/score', element: <ScorePage /> },
      { path: '/leaderboard', element: <LeaderboardPage /> },
      { path: '/admin', element: <PlaceholderPage title="Admin" state="Velg en turnering" /> },
    ],
  },
])
