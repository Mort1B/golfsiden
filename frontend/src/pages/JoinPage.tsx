import { useParams } from 'react-router-dom'
import { JoinExperience } from '../features/invitations/JoinExperience'

export function JoinPage() {
  const { invitationId = '' } = useParams()
  return (
    <main className="join-page">
      <div className="join-shell"><a className="join-brand" href="/">Guttas Golf</a><JoinExperience invitationId={invitationId} /></div>
    </main>
  )
}
