import { useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { CalendarDays, CheckCircle2, LogIn } from 'lucide-react'
import { Link } from 'react-router-dom'
import {
  acceptInvitation,
  invitationKeys,
  previewInvitation,
  registerInvitation,
  type InvitationAcceptanceResult,
  type InvitationRegistrationInput,
} from '../../api/invitations'
import { authKeys } from '../../api/auth'
import { tournamentKeys } from '../../api/tournaments'
import { isCanonicalUuid } from '../../api/decoder'
import { useAuth } from '../auth/authContext'
import { InlineSignIn, RegistrationForm } from './JoinForms'
import { joinErrorMessage, previewFailure, type PreviewFailure } from './joinMessages'
import { clearInvitationFragment, parseInvitationSecret } from './secret'

interface JoinExperienceProps {
  invitationId: string
}

type JoinSuccess = InvitationAcceptanceResult

function formatDate(date: string): string {
  return new Intl.DateTimeFormat('nb-NO', { day: 'numeric', month: 'long', year: 'numeric' })
    .format(new Date(`${date}T12:00:00`))
}

const failureCopy: Record<PreviewFailure, { title: string; message: string }> = {
  invalid: { title: 'Ugyldig invitasjon', message: 'Kontroller at du har åpnet hele lenken.' },
  expired: { title: 'Invitasjonen har utløpt', message: 'Be en turneringsadministrator om en ny lenke.' },
  revoked: { title: 'Invitasjonen er trukket tilbake', message: 'Be en turneringsadministrator om en ny lenke.' },
  exhausted: { title: 'Invitasjonen er brukt opp', message: 'Maks antall påmeldinger for lenken er nådd.' },
  closed: { title: 'Påmeldingen er stengt', message: 'Turneringen er ikke åpen for flere påmeldinger.' },
  retryable: { title: 'Kunne ikke hente invitasjonen', message: 'Kontroller forbindelsen og prøv igjen.' },
}

export function JoinExperience({ invitationId }: JoinExperienceProps) {
  const auth = useAuth()
  const queryClient = useQueryClient()
  const initialSecretStatus = useRef<'missing' | 'malformed' | 'valid' | null>(null)
  const tokenRef = useRef<string | null>(null)
  if (initialSecretStatus.current === null) {
    const parsed = parseInvitationSecret(window.location.hash)
    initialSecretStatus.current = parsed.status
    tokenRef.current = parsed.status === 'valid' ? parsed.token : null
  }
  const [loginPending, setLoginPending] = useState(false)
  const [loginError, setLoginError] = useState<string | null>(null)
  const [success, setSuccess] = useState<JoinSuccess | null>(null)

  const routeValid = isCanonicalUuid(invitationId)
  const preview = useQuery({
    queryKey: invitationKeys.preview(invitationId),
    queryFn: () => {
      const token = tokenRef.current
      if (!token) throw new Error('Invitasjonshemmeligheten mangler.')
      return previewInvitation(invitationId, token)
    },
    enabled: routeValid && tokenRef.current !== null,
    retry: false,
    gcTime: 0,
  })

  const registration = useMutation({
    mutationFn: (input: InvitationRegistrationInput) => {
      const token = tokenRef.current
      if (!token) throw new Error('Invitasjonshemmeligheten mangler.')
      return registerInvitation(invitationId, token, input)
    },
    gcTime: 0,
  })
  const acceptance = useMutation({
    mutationFn: () => {
      const token = tokenRef.current
      const csrfToken = auth.session?.csrf_token
      if (!token || !csrfToken) throw new Error('Du må være logget inn for å bli med.')
      return acceptInvitation(invitationId, token, csrfToken)
    },
    gcTime: 0,
  })

  const finish = async (result: JoinSuccess) => {
    tokenRef.current = null
    clearInvitationFragment()
    setSuccess({
      status: result.status,
      tournament_id: result.tournament_id,
      player_id: result.player_id,
    })
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: tournamentKeys.mineRoot }),
      queryClient.invalidateQueries({ queryKey: tournamentKeys.detail(result.tournament_id) }),
      queryClient.invalidateQueries({ queryKey: tournamentKeys.players(result.tournament_id) }),
    ])
  }

  const register = async (input: InvitationRegistrationInput) => {
    try {
      const result = await registration.mutateAsync(input)
      registration.reset()
      auth.establishSession(result.session)
      queryClient.setQueryData(authKeys.session, result.session)
      await finish(result)
    } catch {
      return
    }
  }

  const accept = async () => {
    try {
      const result = await acceptance.mutateAsync()
      acceptance.reset()
      await finish(result)
    } catch {
      return
    }
  }

  const signIn = async (email: string, password: string) => {
    setLoginPending(true)
    setLoginError(null)
    try {
      await auth.signIn(email, password)
      registration.reset()
    } catch (error) {
      setLoginError(error instanceof Error ? error.message : 'Innlogging mislyktes. Prøv igjen.')
    } finally {
      setLoginPending(false)
    }
  }

  if (!routeValid) return <JoinFailure title="Ugyldig invitasjon" message="Invitasjonsadressen er ikke gyldig." />
  if (initialSecretStatus.current === 'missing') return <JoinFailure title="Lenken mangler en kode" message="Åpne hele invitasjonslenken du fikk tilsendt." />
  if (initialSecretStatus.current === 'malformed') return <JoinFailure title="Ugyldig invitasjonskode" message="Kontroller at du har åpnet hele lenken." />
  if (preview.isPending) return <div className="join-state" role="status">Henter invitasjonen …</div>
  if (preview.error) {
    const failure = previewFailure(preview.error)
    const copy = failureCopy[failure]
    return <JoinFailure title={copy.title} message={copy.message} retry={failure === 'retryable' ? () => void preview.refetch() : undefined} />
  }
  if (success) {
    return (
      <section className="join-success" aria-labelledby="join-success-heading">
        <CheckCircle2 aria-hidden="true" />
        <p className="eyebrow">Påmeldt</p>
        <h1 id="join-success-heading">{success.status === 'already_joined' ? 'Du var allerede med' : 'Du er med!'}</h1>
        <p>{success.status === 'already_joined' ? 'Medlemskapet ditt var allerede aktivt.' : 'Påmeldingen er registrert.'}</p>
        <Link className="invitation-primary" to={`/tournaments/${success.tournament_id}`}>Åpne turneringen</Link>
      </section>
    )
  }

  const mutationPending = registration.isPending || acceptance.isPending
  const mutationError = registration.error ?? acceptance.error
  return (
    <>
      <header className="join-header">
        <p className="eyebrow">Invitasjon til</p>
        <h1>{preview.data.tournament.name}</h1>
        <p className="join-dates"><CalendarDays aria-hidden="true" />{formatDate(preview.data.tournament.start_date)} – {formatDate(preview.data.tournament.end_date)}</p>
      </header>
      {auth.loading && <div className="join-state" role="status">Kontrollerer innlogging …</div>}
      {auth.error && <JoinFailure title="Kunne ikke kontrollere innlogging" message={auth.error.message} retry={() => void auth.retry()} />}
      {!auth.loading && !auth.error && !auth.session && (
        <div className="join-options">
          <RegistrationForm disabled={registration.isPending || loginPending} error={registration.error ? joinErrorMessage(registration.error) : null} onSubmit={register} />
          <InlineSignIn disabled={loginPending || registration.isPending} error={loginError} onSubmit={signIn} />
        </div>
      )}
      {!auth.loading && auth.session && (
        <section className="join-section signed-in" aria-labelledby="accept-heading">
          <h2 id="accept-heading">Bli med som {auth.session.display_name}</h2>
          {auth.session.player_id === null ? (
            <p className="invitation-notice" role="alert">Kontoen din er ikke koblet til en spillerprofil. Be en turneringsadministrator om hjelp.</p>
          ) : (
            <button className="invitation-primary" type="button" disabled={mutationPending} onClick={() => void accept()}><LogIn aria-hidden="true" />{acceptance.isPending ? 'Melder på …' : 'Bli med i turneringen'}</button>
          )}
          {mutationError && <p className="invitation-error" role="alert">{joinErrorMessage(mutationError)}</p>}
        </section>
      )}
    </>
  )
}

function JoinFailure({ title, message, retry }: { title: string; message: string; retry?: () => void }) {
  return (
    <section className="join-failure" role="alert">
      <h1>{title}</h1><p>{message}</p>
      {retry && <button className="invitation-secondary" type="button" onClick={retry}>Prøv igjen</button>}
    </section>
  )
}
