import { useEffect, useMemo, useState, useSyncExternalStore } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useLocation, useParams } from 'react-router-dom'
import { isCanonicalUuid } from '../api/decoder'
import { PublicResultsView } from '../features/resultSharing/PublicResultsView'
import { shareToken } from '../features/resultSharing/format'
import '../features/resultSharing/resultSharing.css'

function subscribeHash(listener: () => void): () => void {
  window.addEventListener('hashchange', listener)
  window.addEventListener('popstate', listener)
  return () => { window.removeEventListener('hashchange', listener); window.removeEventListener('popstate', listener) }
}
function usePrivateDocumentMetadata() {
  useEffect(() => {
    const restore = ['referrer', 'robots'].map(name => {
      const existing = document.querySelector<HTMLMetaElement>(`meta[name="${name}"]`)
      const element = existing ?? document.createElement('meta')
      const previous = element.content
      element.name = name; element.content = name === 'referrer' ? 'no-referrer' : 'noindex, nofollow'
      if (!existing) document.head.append(element)
      return () => { if (existing) element.content = previous; else element.remove() }
    })
    return () => restore.forEach(undo => undo())
  }, [])
}
function Visit({ grantId, token }: { grantId: string; token: string }) {
  const [client] = useState(() => new QueryClient())
  useEffect(() => () => client.clear(), [client])
  return <QueryClientProvider client={client}><PublicResultsView grantId={grantId} token={token} /></QueryClientProvider>
}
export function SharedResultsPage() {
  const { grantId = '' } = useParams()
  const location = useLocation()
  const nativeHash = useSyncExternalStore(subscribeHash, () => window.location.hash)
  // Non-secret visit IDs keep query ownership separate for reloads, Back/Forward
  // and a different capability for the same grant. The fragment remains reusable.
  const visit = useMemo(() => ({ id: `${location.key}:${crypto.randomUUID()}`, grantId, token: shareToken(nativeHash) }), [grantId, nativeHash, location.key])
  usePrivateDocumentMetadata()
  return <main className="public-results-page">
    <header><p className="brand">Guttas Golf</p><p>Delte turneringsresultater</p></header>
    {isCanonicalUuid(grantId) && visit.token ? <Visit key={visit.id} grantId={visit.grantId} token={visit.token} />
      : <p role="alert">Resultatlenken er ikke tilgjengelig. Åpne hele lenken du fikk fra arrangøren.</p>}
  </main>
}
