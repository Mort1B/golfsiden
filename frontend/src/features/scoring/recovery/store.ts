import type { ExpectedScore } from '../../../api/scorecards/conditional'
import type { FourBallInput } from '../../../api/fourBall'
import { queueDatabase, STORAGE_ERROR } from '../offline/database'
import { enqueue, enqueueFourBall, enqueueStableford, queueKey, type FourBallTarget, type QueueTarget, type StablefordTarget } from '../offline/model'
import type { QueueRuntime } from '../offline/runtime'

type Intent = { kind: 'legacy'; target: QueueTarget; value: number }
  | { kind: 'four_ball'; slot: 1 | 2; target: FourBallTarget; value: FourBallInput }
  | { kind: 'stableford'; target: StablefordTarget; value: FourBallInput }
export type LocalDraft = Intent & { key: string; expected: ExpectedScore; sequence: number; predecessor: string | null; saving: boolean; error: string | null; recovery: boolean }

// Only user intent and conditional identity live here. Canonical cards, names,
// permissions, handicap values and totals remain exclusively in Query.
export class ScoreDraftStore {
  private drafts: LocalDraft[] = []
  private listeners = new Set<() => void>()
  private chain = Promise.resolve()
  private sequence = 0
  constructor(readonly accountId: string) {}
  current = () => this.drafts
  subscribe = (listener: () => void) => { this.listeners.add(listener); return () => { this.listeners.delete(listener) } }
  private publish(drafts: LocalDraft[]) { this.drafts = drafts; this.listeners.forEach(listener => listener()) }
  suspend = () => {
    if (this.drafts.some(draft => !draft.recovery)) this.publish(this.drafts.map(draft => ({ ...draft, recovery: true })))
  }
  discard = (key: string) => { this.publish(this.drafts.filter(draft => draft.key !== key || draft.saving)) }
  set = (intent: Intent, expected: ExpectedScore, runtime: QueueRuntime) => {
    if (intent.target.accountId !== this.accountId || !runtime.isCurrent()) return
    const key = queueKey(intent.target), previous = this.drafts.find(draft => draft.key === key)
    if (previous?.recovery) return
    const draft: LocalDraft = { ...intent, key, expected: previous?.expected ?? expected,
      sequence: ++this.sequence, predecessor: previous ? previous.predecessor : runtime.current().items.find(item => item.key === key)?.head.request_id ?? null, saving: true, error: null, recovery: false }
    this.publish([...this.drafts.filter(item => item.key !== key), draft])
    this.persist(draft, runtime)
  }
  retry = (key: string, runtime: QueueRuntime) => {
    const draft = this.drafts.find(item => item.key === key)
    if (!draft || draft.saving || runtime.accountId !== this.accountId || !runtime.isCurrent()) return
    const next = { ...draft, saving: true, error: null }
    this.publish(this.drafts.map(item => item.key === key ? next : item))
    this.persist(next, runtime)
  }
  private persist(draft: LocalDraft, runtime: QueueRuntime) {
    this.chain = this.chain.then(async () => {
      try {
        if (!runtime.isCurrent()) throw new Error('Økten er endret. Endringen beholdes lokalt.')
        let writtenHead: string | null = null
        const latest = this.drafts.find(item => item.key === draft.key)
        const predecessor = latest ? latest.predecessor : draft.predecessor
        const guard = { expectedHead: predecessor, written: (id: string) => { writtenHead = id }, allowed: () => !this.drafts.find(item => item.key === draft.key)?.recovery, subscribe: this.subscribe }
        const current = this.drafts.find(item => item.key === draft.key)
        if (current?.recovery) {
          const item = draft.kind === 'legacy' ? enqueue(null, draft.target, draft.value, draft.expected)
            : draft.kind === 'four_ball' ? enqueueFourBall(null, draft.target, draft.value, draft.expected)
              : enqueueStableford(null, draft.target, draft.value, draft.expected)
          await queueDatabase.retainForReview(item)
        } else if (draft.kind === 'legacy') await queueDatabase.enqueue(draft.target, draft.value, draft.expected, guard)
        else if (draft.kind === 'four_ball') await queueDatabase.enqueueFourBall(draft.target, draft.value, draft.expected, guard)
        else await queueDatabase.enqueueStableford(draft.target, draft.value, draft.expected, guard)
        // The transaction committed: durability does not depend on refetch or connectivity.
        this.publish(this.drafts.filter(item => item.key !== draft.key || item.sequence !== draft.sequence)
          .map(item => item.key === draft.key ? { ...item, predecessor: writtenHead } : item))
        if (runtime.isCurrent()) await runtime.changed()
      } catch (error) {
        this.publish(this.drafts.map(item => item.key === draft.key && item.sequence === draft.sequence
          ? { ...item, saving: false, error: error instanceof Error ? error.message : STORAGE_ERROR } : item))
      }
    })
  }
}
