import { ApiHttpError } from '../../api/http'
import type { ScoreOwner } from '../../api/scorecards'

export interface ScoreIntentScope {
  roundId: string
  owner: ScoreOwner
  holeId: string
}

export type ScoreSyncPhase = 'idle' | 'saving' | 'queued' | 'verifying' | 'synced' | 'failed'

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

interface ScoreCoordinatorDependencies {
  save: (value: number) => Promise<void>
  verify: () => Promise<number | null>
  terminalRefresh: () => Promise<number | null>
}

type Listener = (snapshot: ScoreSyncSnapshot) => void

function failure(error: unknown): ScoreSyncError {
  if (error instanceof ApiHttpError && error.status === 404) {
    return {
      message: 'Scorer-ID finnes ikke. Kontroller VITE_SCORER_USER_ID.',
      retryable: false,
      configuration: true,
    }
  }
  return {
    message: error instanceof Error ? error.message : 'Kunne ikke lagre score',
    retryable: true,
    configuration: false,
  }
}

export class ScoreCoordinator {
  private readonly listeners = new Set<Listener>()
  private inFlight = false
  private snapshot: ScoreSyncSnapshot

  constructor(
    scope: ScoreIntentScope,
    serverValue: number | null,
    private readonly dependencies: ScoreCoordinatorDependencies,
  ) {
    this.snapshot = { scope, serverValue, desiredValue: serverValue, phase: 'idle', error: null }
  }

  current(): ScoreSyncSnapshot {
    return this.snapshot
  }

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener)
    listener(this.snapshot)
    return () => this.listeners.delete(listener)
  }

  setDesired(value: number): void {
    const desiredValue = Math.max(1, Math.min(20, value))
    if (this.snapshot.phase === 'failed') return
    if (!this.inFlight && desiredValue === this.snapshot.serverValue) {
      this.update({ desiredValue, phase: 'synced', error: null })
      return
    }
    this.update({ desiredValue, phase: this.inFlight ? 'queued' : 'saving', error: null })
    if (!this.inFlight) void this.persist()
  }

  retry(): void {
    if (this.snapshot.phase !== 'failed' || !this.snapshot.error?.retryable || this.inFlight) return
    this.update({ phase: 'saving', error: null })
    void this.persist()
  }

  discard(): void {
    if (this.inFlight || this.snapshot.phase !== 'failed') return
    this.update({ desiredValue: this.snapshot.serverValue, phase: 'idle', error: null })
  }

  acceptServerValue(serverValue: number | null): void {
    if (this.inFlight || this.snapshot.phase === 'failed') {
      this.update({ serverValue })
      return
    }
    if (this.snapshot.phase === 'synced' && this.snapshot.desiredValue === serverValue) {
      this.update({ serverValue })
      return
    }
    this.update({ serverValue, desiredValue: serverValue, phase: 'idle', error: null })
  }

  private async persist(): Promise<void> {
    this.inFlight = true
    const submitted = this.snapshot.desiredValue
    if (submitted === null) {
      this.inFlight = false
      return
    }
    try {
      this.update({ phase: 'saving', error: null })
      await this.dependencies.save(submitted)
      this.update({ phase: this.snapshot.desiredValue === submitted ? 'verifying' : 'queued' })
      const verified = await this.dependencies.verify()
      this.update({ serverValue: verified })
      this.inFlight = false
      if (this.snapshot.desiredValue !== verified) {
        this.update({ phase: 'saving' })
        void this.persist()
      } else {
        this.update({ phase: 'synced', error: null })
      }
    } catch (error) {
      if (error instanceof ApiHttpError && error.code === 'round_not_editable') {
        this.update({ desiredValue: this.snapshot.serverValue, phase: 'verifying', error: null })
        try {
          const refreshed = await this.dependencies.terminalRefresh()
          this.update({ serverValue: refreshed, desiredValue: refreshed, phase: 'idle', error: null })
        } catch (refreshError) {
          this.update({ phase: 'failed', error: failure(refreshError) })
        }
      } else {
        this.update({ phase: 'failed', error: failure(error) })
      }
      this.inFlight = false
    }
  }

  private update(change: Partial<Omit<ScoreSyncSnapshot, 'scope'>>): void {
    this.snapshot = { ...this.snapshot, ...change }
    this.listeners.forEach((listener) => listener(this.snapshot))
  }
}

export function hasUnresolvedIntent(snapshot: ScoreSyncSnapshot): boolean {
  return snapshot.phase === 'saving'
    || snapshot.phase === 'queued'
    || snapshot.phase === 'verifying'
    || snapshot.phase === 'failed'
}
