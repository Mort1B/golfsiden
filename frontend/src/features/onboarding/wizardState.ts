import type { OnboardingRequest } from '../../api/onboarding'
import type { TournamentPlanRequest } from '../../api/tournamentCreation'
import type { ScoringFormat } from '../../api/types'
import { parseHandicap } from '../handicap/format'

export interface TournamentDraft {
  name: string
  description: string
  startDate: string
  endDate: string
}

export interface RoundDraft {
  key: string
  name: string
  date: string
  scoringFormat: ScoringFormat
}

export interface CreatorDraft {
  displayName: string
  username: string
  password: string
  handicap: string
}

export interface WizardDraft {
  tournament: TournamentDraft
  rounds: RoundDraft[]
  countedRounds: number | null
  countedRoundsMode: 'all' | 'custom'
  mandatoryRoundKey: string | null
  creator: CreatorDraft
  nextRoundKey: number
}

export function localDateString(date = new Date()): string {
  const year = String(date.getFullYear()).padStart(4, '0')
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

export function createInitialDraft(today = localDateString()): WizardDraft {
  return {
    tournament: { name: '', description: '', startDate: today, endDate: today },
    rounds: [{ key: 'round-1', name: 'Runde 1', date: today, scoringFormat: 'individual_stroke_play' }],
    countedRounds: 1,
    countedRoundsMode: 'all',
    mandatoryRoundKey: null,
    creator: { displayName: '', username: '', password: '', handicap: '' },
    nextRoundKey: 2,
  }
}

export function addRound(draft: WizardDraft): WizardDraft {
  if (draft.rounds.length >= 30) return draft
  const number = draft.rounds.length + 1
  const previous = draft.rounds[draft.rounds.length - 1]
  const countedRounds = draft.countedRoundsMode === 'all'
    ? number
    : draft.countedRounds
  return reconcileOverall({
    ...draft,
    countedRounds,
    nextRoundKey: draft.nextRoundKey + 1,
    rounds: [...draft.rounds, {
      key: `round-${draft.nextRoundKey}`,
      name: `Runde ${number}`,
      date: previous?.date ?? draft.tournament.startDate,
      scoringFormat: previous?.scoringFormat ?? 'individual_stroke_play',
    }],
  })
}

export function removeRound(draft: WizardDraft, key: string): WizardDraft {
  if (draft.rounds.length === 1) return draft
  const rounds = draft.rounds.filter((round) => round.key !== key)
  return reconcileOverall({
    ...draft,
    rounds,
    countedRounds: draft.countedRounds,
    mandatoryRoundKey: draft.mandatoryRoundKey === key ? null : draft.mandatoryRoundKey,
  })
}

export function updateMandatoryRound(draft: WizardDraft, key: string | null): WizardDraft {
  if (key !== null && !draft.rounds.some((round) => round.key === key && round.scoringFormat !== 'singles_match_play')) return draft
  return { ...draft, mandatoryRoundKey: key }
}

export function updateCountedRounds(draft: WizardDraft, countedRounds: number): WizardDraft {
  if (!Number.isInteger(countedRounds) || countedRounds < 1 || countedRounds > eligibleCount(draft.rounds)) return draft
  return {
    ...draft,
    countedRounds,
    countedRoundsMode: countedRounds === eligibleCount(draft.rounds) ? 'all' : 'custom',
  }
}

export function updateRound(
  draft: WizardDraft,
  key: string,
  update: Partial<Omit<RoundDraft, 'key'>>,
): WizardDraft {
  return reconcileOverall({
    ...draft,
    rounds: draft.rounds.map((round) => round.key === key ? { ...round, ...update } : round),
  })
}

export function toOnboardingRequest(draft: WizardDraft): OnboardingRequest {
  return {
    creator: {
      account: { username: draft.creator.username.trim().toLowerCase(), password: draft.creator.password },
      player: {
        display_name: draft.creator.displayName.trim(),
        handicap_index: parseHandicapOrThrow(draft.creator.handicap),
      },
    },
    ...toTournamentPlanRequest(draft),
  }
}

export function toTournamentPlanRequest(draft: WizardDraft): TournamentPlanRequest {
  return {
    tournament: {
      name: draft.tournament.name.trim(),
      description: draft.tournament.description.trim(),
      start_date: draft.tournament.startDate,
      end_date: draft.tournament.endDate,
      counted_rounds: draft.countedRounds,
      mandatory_round_number: draft.mandatoryRoundKey === null
        ? null
        : draft.rounds.findIndex((round) => round.key === draft.mandatoryRoundKey) + 1,
    },
    rounds: draft.rounds.map((round, index) => ({
      round_number: index + 1,
      name: round.name.trim(),
      round_date: round.date,
      scoring_format: round.scoringFormat,
    })),
  }
}

function parseHandicapOrThrow(value: string): number {
  const result = parseHandicap(value)
  if (!result.ok) throw new Error(result.message)
  return result.value
}

export function eligibleCount(rounds: RoundDraft[]): number { return rounds.filter(r => r.scoringFormat !== 'singles_match_play').length }
function reconcileOverall(draft: WizardDraft): WizardDraft {
  const count = eligibleCount(draft.rounds)
  return { ...draft, countedRounds: count === 0 ? null : draft.countedRoundsMode === 'all' ? count : Math.min(draft.countedRounds ?? count, count),
    mandatoryRoundKey: draft.rounds.some(r => r.key === draft.mandatoryRoundKey && r.scoringFormat !== 'singles_match_play') ? draft.mandatoryRoundKey : null }
}
