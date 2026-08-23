import { useId, useState } from 'react'
import { RefreshCw, Save } from 'lucide-react'
import type { CourseCatalogItem, ProviderCourseDetail, ProviderTee } from '../../api/courses'
import { ApiHttpError } from '../../api/http'

interface ReadState<T> {
  data: T | undefined
  pending: boolean
  fetching: boolean
  error: Error | null
  retry: () => void
}

interface Props {
  catalog: ReadState<CourseCatalogItem[]>
  catalogQuery: string
  catalogSearchError: string | null
  catalogIsCurrent: boolean
  detail: ReadState<ProviderCourseDetail>
  providerCourseId: string
  detailIsCurrent: boolean
  saving: boolean
  error: string | null
  onClearError: () => void
  onCatalogQueryChange: (query: string) => void
  onCourseChange: (providerCourseId: string) => void
  onSave: (tee: ProviderTee) => Promise<'saved' | 'tee-stale' | 'failed'>
}

function providerError(error: Error): string {
  if (error instanceof ApiHttpError) {
    if (error.code === 'course_provider_exhausted') return 'Leverandørkvoten er brukt opp. Bruk manuell registrering eller prøv senere.'
    if (error.code === 'course_provider_busy' || error.code === 'course_provider_timeout') return 'Baneleverandøren svarer ikke nå. Prøv igjen eller bruk manuell registrering.'
    if (error.code === 'course_catalog_incomplete') return 'Denne katalogbanen mangler leverandørfakta. Bruk manuell registrering.'
  }
  return 'Kunne ikke hente utslagssteder. Prøv igjen eller bruk manuell registrering.'
}

function teeKey(tee: ProviderTee): string {
  return `${tee.category}:${tee.name}`
}

export function ProviderCoursePicker(props: Props) {
  const pickerId = useId()
  const [selectedTee, setSelectedTee] = useState('')
  const usable = props.catalog.data?.filter((course) => course.provider_status === 'usable' && course.provider_course_id) ?? []
  const unavailable = props.catalog.data?.filter((course) => course.provider_status !== 'usable') ?? []
  const selected = props.detailIsCurrent
    ? props.detail.data?.tees.find((tee) => teeKey(tee) === selectedTee)
    : undefined
  const changeCourse = (value: string) => {
    props.onClearError()
    setSelectedTee('')
    props.onCourseChange(value)
  }
  const changeQuery = (value: string) => {
    props.onClearError()
    setSelectedTee('')
    props.onCourseChange('')
    props.onCatalogQueryChange(value)
  }
  const catalogUnavailable = props.catalog.error && !props.catalog.data
  const catalogStale = Boolean(props.catalog.data) && !props.catalogIsCurrent
  const save = async () => {
    if (!selected) return
    const result = await props.onSave(selected)
    if (result === 'tee-stale') setSelectedTee('')
  }

  return (
    <div className="provider-picker">
      <label><span>Søk i banekatalogen</span><input type="search" value={props.catalogQuery} aria-invalid={Boolean(props.catalogSearchError)} aria-describedby={props.catalogSearchError ? `${pickerId}-search-error` : undefined} onChange={(event) => changeQuery(event.target.value)} disabled={props.saving} /></label>
      {props.catalogSearchError && <p id={`${pickerId}-search-error`} className="course-form-error" role="alert">{props.catalogSearchError}</p>}
      {!props.catalogSearchError && props.catalog.pending && !props.catalog.data && <p className="course-read-state" role="status">Henter banekatalog …</p>}
      {catalogUnavailable && <RetryState message="Kunne ikke hente banekatalogen." retry={props.catalog.retry} />}
      {catalogStale && <div className="course-stale-warning" role="status"><p>{props.catalog.error ? 'Søket kunne ikke oppdateres. Forrige resultat vises, men kan ikke velges.' : 'Forrige søkeresultat vises mens katalogen oppdateres. Det kan ikke velges ennå.'}</p>{props.catalog.error && <button type="button" onClick={props.catalog.retry}><RefreshCw aria-hidden="true" />Prøv igjen</button>}</div>}
      {props.catalog.data && props.catalog.data.length === 0 && props.catalogIsCurrent && <p className="course-read-state">Ingen baner traff søket. Prøv et annet søk eller bruk manuell registrering.</p>}
      {props.catalog.data && props.catalog.data.length > 0 && (
        <label><span>Leverandørbane</span>
          <select value={props.providerCourseId} onChange={(event) => changeCourse(event.target.value)} disabled={props.saving || !props.catalogIsCurrent}>
            <option value="">Velg bane</option>
            {props.catalog.data.map((course) => (
              <option key={course.display_name} value={course.provider_course_id ?? ''} disabled={course.provider_status !== 'usable'}>
                {course.display_name} · {course.country}{course.provider_status === 'usable' ? '' : ' – utilgjengelig'}
              </option>
            ))}
          </select>
        </label>
      )}
      {unavailable.length > 0 && <ul className="catalog-unavailable" aria-label="Utilgjengelige katalogbaner">{unavailable.map((course) => <li key={course.display_name}><strong>{course.display_name}</strong><span>{course.provider_status_detail}</span></li>)}</ul>}
      {props.catalog.data && props.catalogIsCurrent && !usable.length && <p className="course-read-state">Ingen katalogbaner har komplette leverandørfakta ennå. Bruk manuell registrering.</p>}
      {props.providerCourseId && props.detail.pending && <p className="course-read-state" role="status">Henter utslagssteder …</p>}
      {props.providerCourseId && props.detail.fetching && props.detail.data && !props.detailIsCurrent && (
        <p className="course-read-state" role="status">Oppdaterer utslagssteder for valgt bane. Det forrige resultatet kan ikke lagres.</p>
      )}
      {props.providerCourseId && props.detail.error && <RetryState message={providerError(props.detail.error)} retry={props.detail.retry} />}
      {props.detailIsCurrent && props.detail.data && (
        <div className="provider-detail">
          <p><strong>{props.detail.data.course_name}</strong><span>{props.detail.data.club_name}</span></p>
          {!props.detail.data.tees.length ? <p className="course-read-state">Banen har ingen komplette utslagssteder.</p> : (
            <label><span>Utslagssted</span>
              <select value={selectedTee} onChange={(event) => { props.onClearError(); setSelectedTee(event.target.value) }} disabled={props.saving || props.detail.fetching}>
                <option value="">Velg utslagssted</option>
                {props.detail.data.tees.map((tee) => <option key={teeKey(tee)} value={teeKey(tee)}>{tee.name} · {tee.category === 'male' ? 'herre' : 'dame'} · {tee.number_of_holes} hull · par {tee.par_total}</option>)}
              </select>
            </label>
          )}
          {selected && <TeeFacts tee={selected} />}
        </div>
      )}
      {props.error && <p className="course-form-error" role="alert">{props.error}</p>}
      <button className="course-save" type="button" disabled={!selected || props.saving || !props.catalogIsCurrent || props.detail.fetching || Boolean(props.detail.error) || !props.detailIsCurrent} onClick={() => void save()}>
        <Save aria-hidden="true" />{props.saving ? 'Lagrer …' : 'Lagre valgt utslagssted'}
      </button>
    </div>
  )
}

function TeeFacts({ tee }: { tee: ProviderTee }) {
  const length = tee.total_meters > 0
    ? `${tee.total_meters.toLocaleString('nb-NO')} m`
    : tee.total_yards > 0 ? `${tee.total_yards.toLocaleString('nb-NO')} yards` : null
  return (
    <dl className="provider-tee-facts">
      <div><dt>Baneverdi</dt><dd>{tee.course_rating.toLocaleString('nb-NO')}</dd></div>
      <div><dt>Slope</dt><dd>{tee.slope_rating}</dd></div>
      {length && <div><dt>Lengde</dt><dd>{length}</dd></div>}
      <div><dt>Par</dt><dd>{tee.par_total}</dd></div>
      <div><dt>Komplette hull</dt><dd>{tee.holes.length} av {tee.number_of_holes}</dd></div>
    </dl>
  )
}

function RetryState({ message, retry }: { message: string; retry: () => void }) {
  return <div className="course-read-error" role="alert"><p>{message}</p><button type="button" onClick={retry}><RefreshCw aria-hidden="true" />Prøv igjen</button></div>
}
