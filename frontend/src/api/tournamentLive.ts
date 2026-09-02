import { tournamentLiveUrl } from './http'

export const tournamentLiveEventTypes = [
  'tournament',
  'round',
  'team',
  'score',
  'invitation',
  'visibility',
] as const

export type TournamentLiveEventType = typeof tournamentLiveEventTypes[number]
export type TournamentLiveSignal = TournamentLiveEventType | 'open' | 'error'

export interface TournamentLiveSource {
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
    const source = createSource(tournamentLiveUrl(tournamentId), { withCredentials: true })
    const created: SharedSubscription = { source, listeners: new Set(), disposalVersion: 0 }
    const notify = (signal: TournamentLiveSignal) => {
      for (const listener of created.listeners) listener(signal)
    }
    for (const eventType of tournamentLiveEventTypes) {
      source.addEventListener(eventType, () => notify(eventType))
    }
    source.addEventListener('open', () => notify('open'))
    source.addEventListener('error', () => notify('error'))
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
      subscriptions.delete(key)
    })
  }
}
