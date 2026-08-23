import { useRef, useState } from 'react'
import { MapPin } from 'lucide-react'
import type { ManualCourseSelection } from '../../api/courses'
import type { Round } from '../../api/types'
import { StatusBadge } from '../../ui/StatusBadge'
import { configurationErrorMessage, configurationFailure } from './courseConfiguration'
import { ManualCourseForm } from './ManualCourseForm'
import { ProviderCoursePicker } from './ProviderCoursePicker'
import { useCourseConfiguration } from './useCourseConfiguration'

interface Props { tournamentId: string; rounds: Round[] }

function roundName(round: Round): string {
  return `Runde ${round.round_number}: ${round.name}`
}

export function CourseConfigurationSection({ tournamentId, rounds }: Props) {
  const [expandedRoundId, setExpandedRoundId] = useState<string | null>(null)
  return (
    <div className="round-course-configurations">
      {rounds.map((round) => <RoundCourseConfiguration
        key={round.id} tournamentId={tournamentId} round={round}
        expanded={expandedRoundId === round.id}
        onToggle={() => setExpandedRoundId((current) => current === round.id ? null : round.id)}
        onCollapse={() => setExpandedRoundId((current) => current === round.id ? null : current)}
      />)}
    </div>
  )
}

interface RoundConfigurationProps {
  tournamentId: string
  round: Round
  expanded: boolean
  onToggle: () => void
  onCollapse: () => void
}

function RoundCourseConfiguration({ tournamentId, round, expanded, onToggle, onCollapse }: RoundConfigurationProps) {
  const toggleRef = useRef<HTMLButtonElement>(null)
  const [mode, setMode] = useState<'provider' | 'manual'>('provider')
  const [providerCourseId, setProviderCourseId] = useState('')
  const [catalogQuery, setCatalogQuery] = useState('')
  const [receipt, setReceipt] = useState<string | null>(null)
  const state = useCourseConfiguration({ tournamentId, round, providerCourseId, catalogQuery, expanded })
  const failure = configurationFailure(state.mutation.error)
  const effectivelyDraft = round.status === 'draft' && failure !== 'not-draft'
  const collapseAfterSave = () => {
    onCollapse()
    requestAnimationFrame(() => toggleRef.current?.focus())
  }
  const changeMode = (nextMode: 'provider' | 'manual') => {
    state.mutation.reset()
    setMode(nextMode)
  }
  const saveManual = async (selection: ManualCourseSelection) => {
    setReceipt(null)
    const { configured } = await state.save(selection)
    if (configured) {
      setReceipt(`${roundName(configured)} er lagret med ${configured.course_name} · ${configured.tee_name}.`)
      collapseAfterSave()
    }
    return configured !== null
  }
  const saveProvider = async (tee: Parameters<typeof state.saveProvider>[0]) => {
    setReceipt(null)
    const result = await state.saveProvider(tee)
    const { configured } = result
    if (configured) {
      setReceipt(`${roundName(configured)} er lagret med ${configured.course_name} · ${configured.tee_name}.`)
      collapseAfterSave()
    }
    return configured ? 'saved' as const : result.failure === 'tee-stale' ? 'tee-stale' as const : 'failed' as const
  }

  return (
    <article className="round-course-card">
      <header><div className="round-course-summary"><strong>{roundName(round)}</strong><span>{round.course_id && round.tee_id ? `${round.course_name} · ${round.tee_name}` : 'Bane og utslagssted er ikke konfigurert'}</span></div><div className="round-course-actions"><StatusBadge status={round.status} />{effectivelyDraft && <button ref={toggleRef} type="button" aria-expanded={expanded} aria-controls={`course-editor-${round.id}`} onClick={onToggle}>{round.course_id && round.tee_id ? 'Endre' : 'Konfigurer'}</button>}</div></header>
      {!effectivelyDraft ? (
        <p className="course-locked"><MapPin aria-hidden="true" />{round.status === 'draft' ? 'Runden ble åpnet et annet sted. Redigering er stengt.' : 'Bare utkast kan endre bane og utslagssted.'}</p>
      ) : (
        <div id={`course-editor-${round.id}`} className="course-editor" hidden={!expanded}>
          <fieldset className="course-mode"><legend>Registreringsmåte</legend>
            <label><input type="radio" name={`course-mode-${round.id}`} checked={mode === 'provider'} onChange={() => changeMode('provider')} disabled={state.mutation.isPending} /> Velg fra katalog</label>
            <label><input type="radio" name={`course-mode-${round.id}`} checked={mode === 'manual'} onChange={() => changeMode('manual')} disabled={state.mutation.isPending} /> Registrer manuelt</label>
          </fieldset>
          {mode === 'provider' ? (
            <ProviderCoursePicker
              catalog={{ data: state.catalog.data, pending: state.catalog.isPending, fetching: state.catalog.isFetching, error: state.catalog.error, retry: () => void state.catalog.refetch() }}
              catalogQuery={catalogQuery}
              catalogSearchError={state.catalogSearch.ok ? null : state.catalogSearch.message}
              catalogIsCurrent={state.catalogIsCurrent}
              detail={{ data: state.detail.data, pending: state.detail.isPending, fetching: state.detail.isFetching, error: state.detail.error, retry: () => void state.detail.refetch() }}
              providerCourseId={providerCourseId} detailIsCurrent={state.detailIsCurrent}
              saving={state.mutation.isPending} error={configurationErrorMessage(state.mutation.error)}
              onClearError={state.mutation.reset}
              onCatalogQueryChange={setCatalogQuery} onCourseChange={setProviderCourseId} onSave={saveProvider}
            />
          ) : (
            <ManualCourseForm holeCount={round.number_of_holes} disabled={state.mutation.isPending} error={configurationErrorMessage(state.mutation.error)} onSave={saveManual} />
          )}
        </div>
      )}
      <p className="course-receipt" aria-live="polite">{receipt}</p>
    </article>
  )
}
