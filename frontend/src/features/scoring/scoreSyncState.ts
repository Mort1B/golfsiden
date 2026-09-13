import type { ScoreOwner } from '../../api/scorecards'

export interface ScoreIntentScope {
  roundId: string
  owner: ScoreOwner
  holeId: string
}

export type ScoreSyncPhase = 'idle' | 'saving' | 'queued' | 'verifying' | 'synced' | 'failed' | 'refreshing' | 'conflict' | 'blocked'

export interface ScoreSyncError {
  message: string
  retryable: boolean
  configuration: boolean
}

export interface ScoreSyncSnapshot {
  scope: ScoreIntentScope
  serverValue: number | null
  desiredValue: number | null
  phase: ScoreSyncPhase
  error: ScoreSyncError | null
}
