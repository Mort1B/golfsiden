import type { ReactNode } from 'react'

export function LoadingState() {
  return <div className="state-message" role="status">Laster …</div>
}

export function ErrorState({ error, onRetry }: { error: Error; onRetry?: () => void }) {
  return (
    <div className="state-message error" role="alert">
      <p>{error.message}</p>
      {onRetry && <button type="button" className="retry-button" onClick={onRetry}>Prøv igjen</button>}
    </div>
  )
}

export function EmptyState({ children }: { children: ReactNode }) {
  return <div className="state-message">{children}</div>
}
