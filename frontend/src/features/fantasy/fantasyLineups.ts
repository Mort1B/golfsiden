import { fantasyApi, type RoundView, type Save } from '../../api/fantasy'
import { ApiHttpError } from '../../api/http'
import type { FantasyDraft, FantasyRecovery } from './fantasyDrafts'
import type { useFantasyAction } from './useFantasy'

export function validFantasyDraft(draft: FantasyDraft | null | undefined, view: RoundView): draft is FantasyDraft {
  return !!draft && draft.picks.length === 4 && new Set(draft.picks).size === 4
    && draft.picks.includes(draft.captain) && draft.picks.every(p => view.eligible_players.includes(p))
}
export function openFantasyRound(view: RoundView): boolean {
  return view.entered && view.selection_availability === 'open' && !view.window.locked_at
}
export function submitFantasyLineup(
  tournament: string, round: string, body: Save, wasUncertain: boolean,
  action: ReturnType<typeof useFantasyAction>, update: (round: string, patch: Partial<FantasyRecovery>) => void,
) {
  return action.execute(s => {
    update(round, { uncertain: body })
    return fantasyApi.save(tournament, round, s.user_id, body, s.csrf_token)
  }, receipt => update(round, { uncertain: null, draft: null, rejected: null, receipt }), error => {
    if (error instanceof ApiHttpError && error.code === 'fantasy_revision_conflict') {
      update(round, { uncertain: null, rejected: body }); return
    }
    if (!(error instanceof ApiHttpError) || error.status >= 500) update(round, { uncertain: body })
    else if (!wasUncertain || error.code === 'fantasy_closed') update(round, { uncertain: null })
  }, { kind: 'round', round })
}
