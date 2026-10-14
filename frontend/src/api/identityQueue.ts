import type { AuthSession } from './auth'

interface IdentityObserver {
  started: (version: number) => void
  settled: (session: AuthSession | null | undefined, version: number) => Promise<void>
}

let version = 0
let settledVersion = 0
let deliveredVersion = 0
let tail: Promise<void> = Promise.resolve()
const observers = new Set<IdentityObserver>()

export const identityVersion = (): number => version
export const settledIdentityVersion = (): number => settledVersion
export function acceptIdentityPublication(reconciledVersion?: number): void {
  if (deliveredVersion === version || reconciledVersion === version) settledVersion = version
}
export const identityChanged = (): Error => new Error('Innloggingen er endret. Kontroller kontoen og prøv igjen.')

export function observeIdentity(observer: IdentityObserver): () => void {
  observers.add(observer)
  return () => { observers.delete(observer) }
}

export async function waitForIdentity(): Promise<void> {
  let pending: Promise<void>
  do { pending = tail; await pending } while (pending !== tail)
}

// This queue is intentionally in-memory and outlives route components. Waiting
// for response bodies also orders the browser's application of Set-Cookie.
export function coordinateIdentity<T>(
  operation: () => Promise<T>,
  sessionOf: (result: T) => AuthSession | null,
): Promise<T> {
  const intent = ++version
  for (const observer of observers) observer.started(intent)
  const result = tail.then(operation).then(value => {
    deliveredVersion = intent
    if (observers.size === 0 && intent === version) settledVersion = intent
    return value
  })
  const settle = async (session: AuthSession | null | undefined) => {
    // Let accepted mutation continuations retain their receipt/navigation before
    // authoritative publication can change the route's owning branch.
    await new Promise<void>(resolve => setTimeout(resolve, 0))
    if (intent !== version) return
    await Promise.all([...observers].map(observer => observer.settled(session, intent)))
    if (intent === version) settledVersion = intent
  }
  tail = result.then(value => settle(sessionOf(value)), () => settle(undefined))
  return result
}
