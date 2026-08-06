import type { ReactNode } from 'react'

export function LoadingState() {
  return <div className="state-message" role="status">Laster …</div>
}

export function ErrorState({ error }: { error: Error }) {
  return <div className="state-message error" role="alert">{error.message}</div>
}

export function EmptyState({ children }: { children: ReactNode }) {
  return <div className="state-message">{children}</div>
}
