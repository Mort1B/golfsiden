import { isCancelledError, type QueryClient } from '@tanstack/react-query'
import type { ScoringScorecard } from '../../../api/scorecards'
import type { FourBallScoringCard } from '../../../api/fourBall'
import { stablefordApi, type StablefordScoringCard } from '../../../api/stableford'
import { fourBallApi } from '../../../api/fourBall'
import { api } from '../../../api/client'
import { ApiHttpError } from '../../../api/http'
import { scoreRequest } from '../../../api/scorecards/timeout'
import { subscribePageReturn } from '../../../api/pageReturn'
import { privateWorkspaceKeys } from '../../../api/privateWorkspace'
import { scoringKeys } from '../../../api/scorecards'
import { invalidateScorecard, invalidateScoreDependents } from '../queries'
import { queueDatabase, STORAGE_ERROR } from './database'
import { hasLease, pendingOwner, REQUEST_MS, type PendingScore } from './model'

export interface RefreshingScore { item: PendingScore; id: string; retryAt: number; failed: boolean }
export interface QueueSnapshot { items: PendingScore[]; refreshing: RefreshingScore[]; loading: boolean; error: string | null; online: boolean }
export class QueueRuntime {
  private snapshot: QueueSnapshot = { items: [], refreshing: [], loading: true, error: null, online: navigator.onLine }
  private listeners = new Set<() => void>()
  private active = false
  private running = false
  private channel: BroadcastChannel | null = null
  private controller: AbortController | null = null
  constructor(readonly accountId: string, private readonly csrfToken: string, private readonly client: QueryClient,
    private readonly ownsSession: () => boolean) {}
  current = (): QueueSnapshot => this.snapshot
  subscribe = (listener: () => void): (() => void) => { this.listeners.add(listener); return () => this.listeners.delete(listener) }
  isCurrent = (): boolean => this.active && this.ownsSession()
  private update(change: Partial<QueueSnapshot>): void {
    if (!this.isCurrent()) return
    this.snapshot = { ...this.snapshot, ...change }; this.listeners.forEach(listener => listener())
  }
  start(): () => void {
    this.active = true
    if (typeof BroadcastChannel !== 'undefined') {
      this.channel = new BroadcastChannel('golf-pending-scores')
      this.channel.onmessage = () => { void this.wake() }
    }
    const stopReturn = subscribePageReturn(() => { void this.wake() })
    const offline = () => this.update({ online: false })
    window.addEventListener('offline', offline)
    const timer = window.setInterval(() => { void this.wake() }, 2000)
    void this.wake()
    return () => {
      this.active = false; this.controller?.abort(); this.channel?.close(); this.channel = null
      stopReturn(); window.removeEventListener('offline', offline); window.clearInterval(timer)
    }
  }
  changed = async (): Promise<void> => {
    this.channel?.postMessage('changed')
    await this.reload()
    void this.wake()
  }
  private async reload(): Promise<PendingScore[]> {
    try {
      const items = await queueDatabase.list(this.accountId)
      const removed = this.snapshot.items.filter(previous => !items.some(item => item.key === previous.key))
      const refreshing = [...this.snapshot.refreshing.filter(previous => !removed.some(item => item.key === previous.item.key)),
        ...removed.map(item => ({ item, id: crypto.randomUUID(), retryAt: 0, failed: false }))]
      this.update({ items, refreshing, loading: false, error: null, online: navigator.onLine })
      return items
    } catch {
      this.update({ loading: false, error: STORAGE_ERROR })
      return []
    }
  }
  wake = async (): Promise<void> => {
    if (!this.isCurrent()) return
    if (this.running) { await this.reload(); return }
    this.running = true
    try {
      const items = await this.reload()
      if (!this.isCurrent() || !navigator.onLine) return
      const next = items.find(item => item.phase === 'queued' && !hasLease(item) && item.retryAt <= Date.now())
      if (next) await this.deliver(next)
      else {
        const refresh = this.snapshot.refreshing.find(item => item.retryAt <= Date.now())
        if (refresh) await this.refreshCard(refresh)
      }
    } finally {
      this.running = false
      if (this.isCurrent() && navigator.onLine && this.snapshot.items.some(item => item.phase === 'queued' && !hasLease(item) && item.retryAt <= Date.now())) {
        queueMicrotask(() => { void this.wake() })
      }
    }
  }
  private async deliver(candidate: PendingScore): Promise<void> {
    const leaseId = crypto.randomUUID()
    let item: PendingScore | null = null
    let timer: number | undefined
    try {
      item = await queueDatabase.claim(candidate.key, leaseId)
      if (!item || !this.isCurrent()) return
      await this.reload()
      this.channel?.postMessage('changed')
      if (!this.isCurrent()) return
      const controller = new AbortController(); this.controller = controller
      timer = window.setTimeout(() => controller.abort(), REQUEST_MS)
      const ack = item.protocol === 'four_ball_v1'
        ? await fourBallApi.save(item.roundId, item.head, this.csrfToken, controller.signal)
        : item.protocol === 'stableford_v1' ? await stablefordApi.save(item.roundId, item.head, this.csrfToken, controller.signal)
        : await api.saveConditionalScore(item.roundId, item.head, this.csrfToken, controller.signal)
      if (!this.isCurrent()) return
      await queueDatabase.acknowledge(item.key, ack)
      if (!this.isCurrent()) return
      await this.changed()
      const deliveredKey = item.key
      const refresh = this.snapshot.refreshing.find(value => value.item.key === deliveredKey)
      if (refresh) await this.refreshCard(refresh)
    } catch (error) {
      if (!this.isCurrent()) return
      if (!item) { this.update({ error: STORAGE_ERROR }); return }
      const phase = error instanceof ApiHttpError && error.code === 'score_version_conflict' ? 'conflict'
        : error instanceof ApiHttpError && error.status >= 400 && error.status < 500 && error.status !== 429 ? 'blocked' : 'queued'
      try { await queueDatabase.fail(item.key, item.head.request_id, leaseId, phase); await this.changed() }
      catch { this.update({ error: STORAGE_ERROR }) }
      if (phase === 'blocked' && this.isCurrent()) {
        this.client.removeQueries({ queryKey: scoringKeys.scoring(this.accountId, item.roundId, pendingOwner(item)), exact: true })
        void this.client.invalidateQueries({ queryKey: privateWorkspaceKeys.scoreAccess(this.accountId, item.roundId), exact: true })
        void this.client.invalidateQueries({ queryKey: privateWorkspaceKeys.completion(this.accountId, item.roundId), exact: true })
      }
    } finally { window.clearTimeout(timer); this.controller = null }
  }
  private async refreshCard(refresh: RefreshingScore): Promise<void> {
    const item = refresh.item
    if (!this.isCurrent()) return
    await invalidateScorecard(this.client, this.accountId, item.roundId, pendingOwner(item))
    if (!this.isCurrent()) return
    const key = scoringKeys.scoring(this.accountId, item.roundId, pendingOwner(item))
    try {
      await this.client.cancelQueries({ queryKey: key, exact: true })
      if (!this.isCurrent()) return
      await this.client.fetchQuery({ queryKey: key, staleTime: 0, retry: false,
        queryFn: async ({ signal }) => {
          const card = await scoreRequest<ScoringScorecard | FourBallScoringCard | StablefordScoringCard>(child => item.protocol === 'four_ball_v1'
            ? fourBallApi.scoring(item.roundId, item.sideId, child) : item.protocol === 'stableford_v1' ? stablefordApi.scoring(item.roundId, item.owner.id, child) : api.scorecardScoring(item.roundId, item.owner, child), signal)
          if (!this.isCurrent()) throw new Error('Økten er endret')
          return card
        } })
      this.update({ refreshing: this.snapshot.refreshing.filter(value => value.id !== refresh.id) })
    } catch (error) {
      if (error instanceof ApiHttpError && error.status >= 400 && error.status < 500 && error.status !== 429) {
        this.update({ refreshing: this.snapshot.refreshing.filter(value => value.id !== refresh.id) })
        if (this.isCurrent()) {
          this.client.removeQueries({ queryKey: key, exact: true })
          void this.client.invalidateQueries({ queryKey: privateWorkspaceKeys.scoreAccess(this.accountId, item.roundId), exact: true })
          void this.client.invalidateQueries({ queryKey: privateWorkspaceKeys.completion(this.accountId, item.roundId), exact: true })
        }
        return
      }
      // A live refetch can replace this query. Keep verification pending, but
      // retry on the next wake instead of applying a network-failure backoff.
      const cancelled = isCancelledError(error)
      this.update({ refreshing: this.snapshot.refreshing.map(value => value.id === refresh.id
        ? { ...value, failed: !cancelled, retryAt: cancelled ? 0 : Date.now() + 15_000 } : value) })
    }
    if (this.isCurrent()) void invalidateScoreDependents(this.client, this.accountId, item.roundId, item.tournamentId)
  }
}
