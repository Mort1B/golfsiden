import { createContext, useContext } from 'react'
import type { AuthSession } from '../../api/auth'

export interface AuthContextValue {
  session: AuthSession | null
  loading: boolean
  error: Error | null
  signIn: (email: string, password: string) => Promise<AuthSession>
  establishSession: (session: AuthSession) => void
  signOut: () => Promise<void>
  retry: () => Promise<void>
}

export const AuthContext = createContext<AuthContextValue | null>(null)

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext)
  if (!context) throw new Error('useAuth must be used inside AuthProvider')
  return context
}
