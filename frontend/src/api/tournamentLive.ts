import { tournamentLiveUrl } from './http'
import { subscribePageReturn } from './pageReturn'

export const tournamentLiveEventTypes = [
  'tournament',
  'round',
  'team',
  'score',
  'invitation',
  'visibility',
] as const

export type TournamentLiveEventType = typeof tournamentLiveEventTypes[number]
export type TournamentLiveSignal = TournamentLiveEventType | 'open' | 'error' | 'resume'

export interface TournamentLiveSource {
  readonly readyState: number
  addEventListener(type: string, listener: EventListener): void
  close(): void
}

export type TournamentLiveSourceFactory = (
  url: string,
  init: EventSourceInit,
) => TournamentLiveSource

interface SharedSubscription {
  source: TournamentLiveSource
  listeners: Set<(signal: TournamentLiveSignal) => void>
  disposalVersion: number
  resume: () => void
  stopReturns: () => void
}

const subscriptions = new Map<string, SharedSubscription>()

const nativeSource: TournamentLiveSourceFactory = (url, init) => new EventSource(url, init)

export function subscribeTournamentLive(
  userId: string,
  tournamentId: string,
  onInvalidate: (signal: TournamentLiveSignal) => void,
  createSource: TournamentLiveSourceFactory = nativeSource,
): () => void {
  if (userId.length === 0 || tournamentId.length === 0) return () => undefined

  const key = `${userId}:${tournamentId}`
  let subscription = subscriptions.get(key)
  if (!subscription) {
    const makeSource = () => createSource(tournamentLiveUrl(tournamentId), { withCredentials: true })
    const created: SharedSubscription = { source: makeSource(), listeners: new Set(), disposalVersion: 0,
      resume: () => undefined, stopReturns: () => undefined }
    const notify = (signal: TournamentLiveSignal) => {
      for (const listener of created.listeners) listener(signal)
    }
    let resumedSource: TournamentLiveSource | null = null
    const attach = (source: TournamentLiveSource) => {
      for (const signal of [...tournamentLiveEventTypes, 'open', 'error'] as const) {
        source.addEventListener(signal, () => {
          if (source !== created.source || created.listeners.size === 0) return
          if (signal === 'error' || signal === 'open') resumedSource = null
          notify(signal)
        })
      }
    }
    created.resume = () => {
      if (created.listeners.size === 0) return
      notify('resume')
      // Do not interrupt a healthy stream or a restart already connecting.
      if (created.source.readyState === 1
        || (created.source.readyState === 0 && resumedSource === created.source)) return
      created.source.close()
      created.source = makeSource()
      resumedSource = created.source
      attach(created.source)
    }
    attach(created.source)
    created.stopReturns = typeof document === 'undefined' ? () => undefined : subscribePageReturn(created.resume)
    subscriptions.set(key, created)
    subscription = created
  }

  subscription.disposalVersion += 1
  subscription.listeners.add(onInvalidate)
  const activeSubscription = subscription
  let active = true
  return () => {
    if (!active) return
    active = false
    activeSubscription.listeners.delete(onInvalidate)
    if (activeSubscription.listeners.size > 0) return
    const disposalVersion = ++activeSubscription.disposalVersion
    queueMicrotask(() => {
      if (activeSubscription.listeners.size > 0
        || activeSubscription.disposalVersion !== disposalVersion) return
      activeSubscription.source.close()
      activeSubscription.stopReturns()
      subscriptions.delete(key)
    })
  }
}

export function resumeTournamentLive(userId: string, tournamentId: string): void {
  subscriptions.get(`${userId}:${tournamentId}`)?.resume()
}
