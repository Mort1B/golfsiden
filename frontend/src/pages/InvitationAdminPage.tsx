import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ChevronLeft } from 'lucide-react'
import { Link, useParams } from 'react-router-dom'
import {
  buildInvitationUrl,
  issueInvitation,
  listInvitations,
  revokeInvitation,
  rotateInvitation,
  type InvitationIssueInput,
} from '../api/invitations'
import { isCanonicalUuid } from '../api/decoder'
import { tournamentApi, tournamentKeys } from '../api/tournaments'
import { ApiHttpError } from '../api/http'
import { privateWorkspaceKeys } from '../api/privateWorkspace'
import { InvitationIssueForm } from '../features/invitations/InvitationIssueForm'
import { InvitationList } from '../features/invitations/InvitationList'
import { OneTimeInvitationLink } from '../features/invitations/OneTimeInvitationLink'
import {
  revealedAfterRevoke,
  revealedInvitationKey,
  type RevealedInvitation,
} from '../features/invitations/adminState'
import { useAuth } from '../features/auth/authContext'
import { ErrorState, LoadingState } from '../ui/AsyncState'

interface ActionError { scope: 'issue' | 'list'; message: string }

function adminError(error: unknown): string {
  if (error instanceof ApiHttpError) {
    if (error.status === 403) return 'Du har ikke administratortilgang til denne turneringen.'
    if (error.code === 'invitation_rotation_conflict') return 'Invitasjonen ble allerede rotert. Oppdater listen og prøv igjen.'
    if (error.code === 'invitation_expired' || error.code === 'invitation_revoked') return 'Bare aktive invitasjoner kan roteres.'
  }
  return error instanceof Error ? error.message : 'Handlingen mislyktes. Prøv igjen.'
}

export function InvitationAdminPage() {
  const { tournamentId = '' } = useParams()
  const auth = useAuth()
  const queryClient = useQueryClient()
  const userId = auth.session?.user_id ?? ''
  const memberships = useQuery({ queryKey: tournamentKeys.mine(userId), queryFn: tournamentApi.mine, enabled: userId.length > 0 })
  const membership = memberships.data?.find((entry) => entry.tournament.id === tournamentId)
  const authorized = membership?.role === 'admin'
  const invitations = useQuery({
    queryKey: privateWorkspaceKeys.invitations(userId, tournamentId),
    queryFn: () => listInvitations(tournamentId),
    enabled: authorized && isCanonicalUuid(tournamentId),
  })
  const [revealed, setRevealed] = useState<RevealedInvitation | null>(null)
  const [pendingId, setPendingId] = useState<string | null>(null)
  const [actionError, setActionError] = useState<ActionError | null>(null)

  const issue = useMutation({
    mutationFn: (input: InvitationIssueInput) => issueInvitation(tournamentId, input, auth.session?.csrf_token ?? ''),
    gcTime: 0,
  })
  const rotate = useMutation({
    mutationFn: (invitationId: string) => rotateInvitation(tournamentId, invitationId, auth.session?.csrf_token ?? ''),
    gcTime: 0,
  })
  const revoke = useMutation({
    mutationFn: (invitationId: string) => revokeInvitation(tournamentId, invitationId, auth.session?.csrf_token ?? ''),
  })

  const refreshList = () => queryClient.invalidateQueries({
    queryKey: privateWorkspaceKeys.invitations(userId, tournamentId),
    exact: true,
  })

  const handleIssue = async (input: InvitationIssueInput) => {
    setActionError(null)
    try {
      const result = await issue.mutateAsync(input)
      setRevealed({ invitationId: result.id, token: result.token })
      issue.reset()
      await refreshList()
    } catch (error) {
      setActionError({ scope: 'issue', message: adminError(error) })
    }
  }

  const handleRotate = async (invitationId: string) => {
    setPendingId(invitationId)
    setActionError(null)
    try {
      const result = await rotate.mutateAsync(invitationId)
      setRevealed({ invitationId: result.id, token: result.token })
      rotate.reset()
      await refreshList()
    } catch (error) {
      setActionError({ scope: 'list', message: adminError(error) })
    } finally {
      setPendingId(null)
    }
  }

  const handleRevoke = async (invitationId: string) => {
    setPendingId(invitationId)
    setActionError(null)
    try {
      await revoke.mutateAsync(invitationId)
      setRevealed((current) => revealedAfterRevoke(current, invitationId))
      await refreshList()
    } catch (error) {
      setActionError({ scope: 'list', message: adminError(error) })
    } finally {
      setPendingId(null)
    }
  }

  if (!isCanonicalUuid(tournamentId)) return <AdminState title="Ugyldig turnering" />
  if (memberships.isPending) return <section className="page"><LoadingState /></section>
  if (memberships.error) return <section className="page"><ErrorState error={memberships.error} onRetry={() => void memberships.refetch()} /></section>
  if (!authorized || !membership) return <AdminState title="Ingen tilgang" message="Bare turneringsadministratorer kan administrere invitasjoner." />

  return (
    <section className="page invitation-admin-page">
      <header className="detail-header">
        <Link to={`/tournaments/${tournamentId}`} className="back-button" aria-label="Tilbake til turneringen"><ChevronLeft /></Link>
        <div><p className="brand">{membership.tournament.name}</p><h1>Invitasjoner</h1></div>
      </header>
      {revealed && <OneTimeInvitationLink key={revealedInvitationKey(revealed)} url={buildInvitationUrl(window.location.origin, revealed.invitationId, revealed.token)} onDismiss={() => setRevealed(null)} />}
      <InvitationIssueForm disabled={issue.isPending || pendingId !== null} error={actionError?.scope === 'issue' ? actionError.message : null} onIssue={handleIssue} />
      <section className="invitation-admin-section" aria-labelledby="invitation-list-heading">
        <div className="invitation-section-heading"><div><p className="eyebrow">Oversikt</p><h2 id="invitation-list-heading">Invitasjonslenker</h2></div><span>{invitations.data?.length ?? 0}</span></div>
        {invitations.isPending && <LoadingState />}
        {invitations.error && <ErrorState error={invitations.error} onRetry={() => void invitations.refetch()} />}
        {actionError?.scope === 'list' && <p className="invitation-error" role="alert">{actionError.message}</p>}
        {invitations.data && <InvitationList invitations={invitations.data} pendingId={pendingId} onRotate={handleRotate} onRevoke={handleRevoke} />}
      </section>
    </section>
  )
}

function AdminState({ title, message }: { title: string; message?: string }) {
  return <section className="page"><div className="state-message error" role="alert"><h1>{title}</h1>{message && <p>{message}</p>}</div></section>
}
