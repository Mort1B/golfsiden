import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { presetApi, presetKey } from '../../api/coursePresets'
import type { ManualCourseSelection } from '../../api/courses'
import { useAuth } from '../auth/authContext'
import { ErrorState, LoadingState } from '../../ui/AsyncState'
import './saved-courses.css'

interface Props { tournamentId: string; enabled: boolean; saving: boolean; error: string | null; onSave: (selection: ManualCourseSelection) => Promise<boolean> }
export function SavedCoursePicker({ tournamentId, enabled, saving, error, onSave }: Props) {
  const userId = useAuth().session?.user_id ?? ''
  const query = useQuery({ queryKey: presetKey(userId, tournamentId), queryFn: () => presetApi.list(tournamentId), enabled: enabled && !!userId })
  const [selectedId, setSelectedId] = useState('')
  const selected = query.error ? undefined : query.data?.find((item) => item.id === selectedId)?.selection
  return <div className="saved-course-picker provider-picker">
    <p>Lagrede banedata fra turneringsadministrator. Velg bane og kontroller utslagsstedet før lagring. Ingen leverandørtilgang er nødvendig.</p>
    {query.isPending && <LoadingState />}
    {query.error && <ErrorState error={query.error} onRetry={() => void query.refetch()} />}
    {!query.error && query.data?.length === 0 && <p>Ingen lagrede baner er tilgjengelige.</p>}
    {!query.error && query.data && <label><span>Lagret bane</span><select value={selectedId} disabled={!enabled || saving || query.isFetching} onChange={(event) => setSelectedId(event.target.value)}>
      <option value="">Velg bane</option>{query.data.map((item) => <option key={item.id} value={item.id}>{item.selection.course_name}</option>)}
    </select></label>}
    {selected && <>
      <h4>{selected.course_name}</h4>
      <p>{selected.tee.name} · {selected.tee.category === 'male' ? 'Herre' : 'Dame'} · {selected.tee.holes.length} hull · Par {selected.tee.holes.reduce((sum, hole) => sum + hole.par, 0)}</p>
      <p>Baneverdi {selected.tee.course_rating.toLocaleString('nb-NO')} · Slope {selected.tee.slope_rating}</p>
      <p>Lagring setter denne runden til {selected.tee.holes.length} hull. Andre runder og historiske resultater endres ikke.</p>
      <details><summary>Vis par og slagindeks for alle hull</summary><div className="course-hole-scroll"><table><thead><tr><th>Hull</th><th>Par</th><th>Slagindeks</th></tr></thead><tbody>{selected.tee.holes.map((hole, i) => <tr key={i}><td>{i + 1}</td><td>{hole.par}</td><td>{hole.stroke_index}</td></tr>)}</tbody></table></div></details>
    </>}
    {error && <p role="alert">{error}</p>}
    <button className="course-save" type="button" disabled={!selected || !enabled || saving || query.isFetching || !!query.error} onClick={() => { if (selected) void onSave(selected) }}>{saving ? 'Lagrer …' : 'Bruk lagret bane på runden'}</button>
  </div>
}
