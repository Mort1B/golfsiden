import { PublicStandings } from './PublicStandings'
import { usePublicResults } from './usePublicResults'
import { shareUpdated } from './format'

export function PublicResultsView({ grantId, token }: { grantId: string; token: string }) {
  const { query, metric, terminal, refresh, selectMetric } = usePublicResults(grantId, token)
  const board = !terminal && !query.error && query.data && Date.parse(query.data.expires_at) > Date.now() ? query.data : null
  return <>
    <p>Resultatene oppdateres hvert 15. sekund mens siden er synlig.</p>
    {!terminal && <div className="public-result-controls">
      <fieldset><legend>Beregning</legend><div>
        <button type="button" aria-pressed={metric === 'gross'} onClick={() => selectMetric('gross')}>Brutto</button>
        <button type="button" aria-pressed={metric === 'net'} onClick={() => selectMetric('net')}>Netto</button>
      </div></fieldset>
      <button type="button" disabled={query.isFetching} onClick={refresh}>Oppdater resultater</button>
    </div>}
    {terminal ? <p role="alert">Resultatlenken er ikke tilgjengelig. Den kan være utløpt, erstattet eller tilbakekalt.</p>
      : query.error ? <div role="alert"><p>Kunne ikke hente delte resultater. Tidligere resultater er skjult frem til ny kontroll.</p><button type="button" onClick={refresh}>Prøv igjen</button></div>
        : !board ? <p role="status">Kontrollerer lenken og henter resultater …</p>
          : <><p className="public-result-updated" role="status">Sist oppdatert {shareUpdated(query.dataUpdatedAt)}</p><PublicStandings board={board} /></>}
  </>
}
