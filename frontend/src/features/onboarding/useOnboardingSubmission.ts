import { useRef, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { createOnboarding } from '../../api/onboarding'
import { ApiHttpError } from '../../api/http'
import { tournamentKeys, withCreatedTournament, type MyTournament } from '../../api/tournaments'
import type { Round, Tournament } from '../../api/types'
import { useAuth } from '../auth/authContext'
import { toOnboardingRequest, type WizardDraft } from './wizardState'

export interface OnboardingSuccess {
  tournament: Tournament
  rounds: Round[]
  invitation: { id: string; token: string; expiresAt: string }
}

type SubmissionState =
  | { status: 'idle'; error: null; success: null }
  | { status: 'submitting'; error: null; success: null }
  | { status: 'error'; error: string; success: null }
  | { status: 'success'; error: null; success: OnboardingSuccess }

function userMessage(error: unknown): string {
  if (error instanceof ApiHttpError) {
    if (error.code === 'username_already_registered') {
      return 'Brukernavnet er allerede tatt. Velg et annet brukernavn eller logg inn.'
    }
    if (error.code === 'already_authenticated') {
      return 'Du er allerede logget inn. Gå til turneringene dine eller logg ut før du oppretter en ny konto.'
    }
    if (error.status === 400) return 'Opplysningene ble ikke godkjent. Kontroller feltene og prøv igjen.'
  }
  return error instanceof Error ? error.message : 'Turneringen kunne ikke opprettes. Prøv igjen.'
}

export function useOnboardingSubmission(onCreated: () => void) {
  const queryClient = useQueryClient()
  const auth = useAuth()
  const inFlight = useRef(false)
  const [state, setState] = useState<SubmissionState>({ status: 'idle', error: null, success: null })

  const submit = async (draft: WizardDraft): Promise<boolean> => {
    if (inFlight.current) return false
    inFlight.current = true
    setState({ status: 'submitting', error: null, success: null })
    try {
      const response = await createOnboarding(toOnboardingRequest(draft))
      const membership: MyTournament = {
        tournament: response.tournament,
        role: response.creator.tournament_role,
        player_id: response.creator.player_id,
      }
      onCreated()
      auth.establishSession(response.session)
      queryClient.setQueryData<MyTournament[]>(tournamentKeys.mine(response.session.user_id), [membership])
      queryClient.setQueryData(tournamentKeys.detail(response.session.user_id, response.tournament.id), response.tournament)
      queryClient.setQueryData(tournamentKeys.rounds(response.session.user_id, response.tournament.id), response.rounds)
      queryClient.setQueryData<Tournament[]>(tournamentKeys.list(response.session.user_id), (current) =>
        withCreatedTournament(current, response.tournament))
      setState({
        status: 'success',
        error: null,
        success: {
          tournament: response.tournament,
          rounds: response.rounds,
          invitation: {
            id: response.invitation.id,
            token: response.invitation.token,
            expiresAt: response.invitation.expires_at,
          },
        },
      })
      return true
    } catch (error) {
      setState({ status: 'error', error: userMessage(error), success: null })
      return false
    } finally {
      inFlight.current = false
    }
  }

  return { state, submit }
}
