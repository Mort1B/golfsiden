import type { QueryClient } from '@tanstack/react-query'
import { matchApi, matchKeys, type MatchCommand, type MatchScoringCard } from '../../../api/matchPlay'
import { ApiHttpError } from '../../../api/http'
import { invalidateMatchQueries } from '../invalidation'
import { matchDatabase } from './database'
import { matchDraftKey, STORAGE_ERROR, type MatchDraft, type MatchTarget, type Delivery } from './model'
export interface MatchSnapshot { items: MatchDraft[]; loading: boolean; error: string | null; online: boolean }
export class MatchRuntime {
  private snapshot: MatchSnapshot = { items: [], loading: true, error: null, online: navigator.onLine }
  private listeners = new Set<() => void>()
  private active = false
  private running = false
  private channel: BroadcastChannel | null = null
  private controllers = new Set<AbortController>()
  constructor(readonly accountId: string, private csrf: string, private client: QueryClient, private ownsSession: () => boolean) {}
  current = (): MatchSnapshot => this.snapshot
  subscribe = (listener: () => void): (() => void) => { this.listeners.add(listener); return () => this.listeners.delete(listener) }
  isCurrent = (): boolean => this.active && this.ownsSession()
  private update(change: Partial<MatchSnapshot>): void { if (this.isCurrent()) { this.snapshot = { ...this.snapshot, ...change }; this.listeners.forEach(fn => fn()) } }
  start(): () => void {
    this.active = true
    if (typeof BroadcastChannel !== 'undefined') { this.channel = new BroadcastChannel('golf-match-notes-v1'); this.channel.onmessage = () => { void this.wake() } }
    const wake = () => { void this.wake() }
    const offline = () => this.update({ online: false })
    window.addEventListener('online', wake); window.addEventListener('offline', offline); window.addEventListener('focus', wake)
    const timer = window.setInterval(wake, 2000); void this.wake()
    return () => { this.active = false; this.controllers.forEach(c => c.abort()); this.channel?.close(); window.clearInterval(timer); window.removeEventListener('online', wake); window.removeEventListener('offline', offline); window.removeEventListener('focus', wake) }
  }
  private async reload(): Promise<MatchDraft[]> {
    try { const items = await matchDatabase.list(this.accountId); this.update({ items, loading: false, error: null, online: navigator.onLine }); return items }
    catch { this.update({ loading: false, error: STORAGE_ERROR }); return [] }
  }
  changed = async (): Promise<void> => { this.channel?.postMessage('changed'); await this.reload(); void this.wake() }
  private async timed<T>(work: (signal: AbortSignal) => Promise<T>): Promise<T> {
    const controller = new AbortController(); this.controllers.add(controller)
    const timer = window.setTimeout(() => controller.abort(), 12000)
    try { return await work(controller.signal) } finally { window.clearTimeout(timer); this.controllers.delete(controller) }
  }
  private async verify(item: MatchDraft, delivery: Delivery, lease: string): Promise<void> {
    let card: MatchScoringCard
    try { card = await this.timed(signal => matchApi.scoring(item.roundId, item.matchId, signal)) }
    catch (error) {
      if (error instanceof ApiHttpError && error.status === 403) {
        const read = await this.timed(signal => matchApi.read(item.roundId, item.matchId, signal))
        if (!this.isCurrent()) return
        if (read.round_status === 'locked') {
          await matchDatabase.verified(item.key, delivery.request.request_id, lease, true)
          if (!this.isCurrent()) return
          this.client.setQueriesData({ queryKey: matchKeys.read(this.accountId, item.roundId, item.matchId), exact: true }, read)
          await invalidateMatchQueries(this.client, this.accountId, item.roundId, item.tournamentId)
          return
        }
      }
      throw error
    }
    if (!this.isCurrent()) return
    if (BigInt(card.revision) <= BigInt(delivery.request.expected_revision)) throw new Error('Serverens mottak er bekreftet, men gjeldende match er ikke kontrollert ennå.')
    await matchDatabase.verified(item.key, delivery.request.request_id, lease, !!card.finish || card.round_status === 'locked' || card.revision !== String(BigInt(delivery.request.expected_revision) + 1n))
    if (!this.isCurrent()) return
    // Only update existing session-owned queries, never revive a departed account.
    this.client.setQueriesData({ queryKey: matchKeys.scoring(this.accountId, item.roundId, item.matchId), exact: true }, card)
    await invalidateMatchQueries(this.client, this.accountId, item.roundId, item.tournamentId)
  }
  private async deliver(item: MatchDraft, lease: string): Promise<void> {
    const delivery = item.action ?? item.notes[0]; if (!delivery) return
    let acknowledged = delivery.phase === 'acknowledged'
    try {
      if (!acknowledged) {
        await this.timed(signal => matchApi.command(item.roundId, item.matchId, delivery.request, this.csrf, signal))
        if (!this.isCurrent()) return
        acknowledged = true
        await matchDatabase.mark(item.key, delivery.request.request_id, lease, 'acknowledged', null)
      }
      if (this.isCurrent()) await this.verify(item, delivery, lease)
    } catch (error) {
      if (!this.isCurrent()) return
      const definite = error instanceof ApiHttpError && (error.status === 400 || error.status === 409)
      await matchDatabase.mark(item.key, delivery.request.request_id, lease, acknowledged ? 'acknowledged' : definite ? 'blocked' : 'unknown',
        acknowledged ? 'Mottatt av serveren. Gjeldende match må fortsatt kontrolleres.' : definite ? 'Endringen ble avvist. Se gjennom gammel, lokal og gjeldende verdi.' : 'Levering er ukjent. Den opprinnelige forespørselen kontrolleres på nytt.').catch(() => this.update({ error: STORAGE_ERROR }))
    } finally { await matchDatabase.release(item.key, lease).catch(() => this.update({ error: STORAGE_ERROR })); await this.reload(); this.channel?.postMessage('changed') }
  }
  wake = async (): Promise<void> => {
    if (!this.isCurrent()) return
    if (this.running) { await this.reload(); return }
    this.running = true
    try {
      const items = await this.reload()
      if (!this.isCurrent() || !navigator.onLine) return
      const candidate = items.find(item => { const head = item.action ?? item.notes[0]; return head && head.phase !== 'blocked' && item.retryAt <= Date.now() && (!item.lease || item.lease.until <= Date.now()) })
      if (candidate) { const lease = crypto.randomUUID(), item = await matchDatabase.claim(candidate.key, lease); if (item && this.isCurrent()) await this.deliver(item, lease) }
    } catch { this.update({ error: STORAGE_ERROR }) }
    finally { this.running = false }
  }
  onlineAction = async (target: MatchTarget, observed: MatchScoringCard, command: MatchCommand): Promise<void> => {
    if (!this.isCurrent() || !navigator.onLine || this.snapshot.loading || this.snapshot.error) throw new Error('Koble til og kontroller lokale endringer først.')
    const lease = crypto.randomUUID(), key = matchDraftKey(target)
    await matchDatabase.lease(target, lease)
    try {
      const fresh = await this.timed(signal => matchApi.scoring(target.roundId, target.matchId, signal))
      if (!this.isCurrent()) return
      if (fresh.revision !== observed.revision) throw new Error('Matchen er endret. Oppdater og kontroller handlingen på nytt.')
      const request = { request_id: crypto.randomUUID(), expected_revision: fresh.revision, command }
      await matchDatabase.dispatch(key, lease, request)
      if (!this.isCurrent()) return
      const item = (await matchDatabase.list(this.accountId)).find(i => i.key === key)
      if (!item) throw new Error(STORAGE_ERROR)
      await this.deliver(item, lease)
    } finally { await matchDatabase.release(key, lease).catch(() => this.update({ error: STORAGE_ERROR })); await this.changed() }
  }
}
