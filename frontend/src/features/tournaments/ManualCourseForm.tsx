import { useId, useRef, useState, type FormEvent } from 'react'
import { Save } from 'lucide-react'
import type { ManualCourseSelection, TeeCategory } from '../../api/courses'
import {
  createManualDraft,
  resizeManualDraft,
  validateManualDraft,
  type HoleField,
  type ManualCourseDraft,
  type ManualField,
} from './courseConfiguration'

interface Props {
  holeCount: number
  disabled: boolean
  error: string | null
  onSave: (selection: ManualCourseSelection) => Promise<boolean>
}

export function ManualCourseForm({ holeCount, disabled, error, onSave }: Props) {
  const formId = useId()
  const formRef = useRef<HTMLFormElement>(null)
  const [draft, setDraft] = useState<ManualCourseDraft>(() => createManualDraft(holeCount))
  const [visibleHoleCount, setVisibleHoleCount] = useState(holeCount)
  const [validation, setValidation] = useState<ReturnType<typeof validateManualDraft> | null>(null)
  const fieldErrors = validation && !validation.ok ? validation.fieldErrors : {}
  const holeErrors = validation && !validation.ok ? validation.holeErrors : {}

  const field = (key: keyof Omit<ManualCourseDraft, 'holes' | 'category'>, value: string) => {
    setValidation(null)
    setDraft((current) => ({ ...current, [key]: value }))
  }
  const changeHoleCount = (value: string) => {
    setValidation(null)
    const parsed = /^\d+$/.test(value) ? Number(value) : 0
    if (parsed >= 1 && parsed <= 36) setVisibleHoleCount(parsed)
    setDraft((current) => parsed >= 1 && parsed <= 36
      ? resizeManualDraft(current, parsed)
      : { ...current, holeCount: value })
  }
  const hole = (index: number, key: HoleField, value: string) => {
    setValidation(null)
    setDraft((current) => ({
      ...current,
      holes: current.holes.map((item, itemIndex) => itemIndex === index ? { ...item, [key]: value } : item),
    }))
  }
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const result = validateManualDraft(draft)
    setValidation(result)
    if (!result.ok) {
      requestAnimationFrame(() => {
        formRef.current?.querySelector<HTMLElement>(`[data-validation-key="${result.firstInvalid}"]`)?.focus()
      })
      return
    }
    await onSave(result.value)
  }
  const category = (value: string): TeeCategory => value === 'female' ? 'female' : 'male'
  const errorProps = (key: ManualField) => ({
    'aria-invalid': Boolean(fieldErrors[key]),
    'aria-describedby': fieldErrors[key] ? `${formId}-${key}-error` : undefined,
    'data-validation-key': key,
  })

  return (
    <form ref={formRef} className="manual-course-form" aria-busy={disabled} noValidate onSubmit={(event) => void submit(event)}>
      <div className="course-form-grid">
        <label><span>Antall hull</span><input {...errorProps('holeCount')} required inputMode="numeric" value={draft.holeCount} onChange={(event) => changeHoleCount(event.target.value)} disabled={disabled} />{fieldErrors.holeCount && <small id={`${formId}-holeCount-error`} className="field-error">{fieldErrors.holeCount}</small>}</label>
        <label><span>Banenavn</span><input {...errorProps('courseName')} required value={draft.courseName} onChange={(event) => field('courseName', event.target.value)} disabled={disabled} />{fieldErrors.courseName && <small id={`${formId}-courseName-error`} className="field-error">{fieldErrors.courseName}</small>}</label>
        <label><span>Sted <small>(valgfritt)</small></span><input {...errorProps('location')} value={draft.location} onChange={(event) => field('location', event.target.value)} disabled={disabled} />{fieldErrors.location && <small id={`${formId}-location-error`} className="field-error">{fieldErrors.location}</small>}</label>
        <label><span>Kategori</span><select value={draft.category} onChange={(event) => setDraft((current) => ({ ...current, category: category(event.target.value) }))} disabled={disabled}><option value="male">Herre</option><option value="female">Dame</option></select></label>
        <label><span>Utslagssted</span><input {...errorProps('teeName')} required value={draft.teeName} onChange={(event) => field('teeName', event.target.value)} disabled={disabled} />{fieldErrors.teeName && <small id={`${formId}-teeName-error`} className="field-error">{fieldErrors.teeName}</small>}</label>
        <label><span>Baneverdi</span><input {...errorProps('courseRating')} required inputMode="decimal" placeholder="71,2" value={draft.courseRating} onChange={(event) => field('courseRating', event.target.value)} disabled={disabled} />{fieldErrors.courseRating && <small id={`${formId}-courseRating-error`} className="field-error">{fieldErrors.courseRating}</small>}</label>
        <label><span>Slope</span><input {...errorProps('slopeRating')} required inputMode="numeric" value={draft.slopeRating} onChange={(event) => field('slopeRating', event.target.value)} disabled={disabled} />{fieldErrors.slopeRating && <small id={`${formId}-slopeRating-error`} className="field-error">{fieldErrors.slopeRating}</small>}</label>
      </div>
      <fieldset className="manual-holes">
        <legend>Hull</legend>
        <p>Alle {visibleHoleCount} hull må ha par og en unik slagindeks fra 1 til {visibleHoleCount}. Valgfri avstand oppgis i yards og lagres ikke når feltet står tomt.</p>
        <div className="hole-grid-heading" aria-hidden="true"><span>Hull</span><span>Par</span><span>Slagindeks</span><span>Avstand (yards)</span></div>
        {draft.holes.slice(0, visibleHoleCount).map((item, index) => <HoleRow key={index} formId={formId} index={index} value={item} errors={holeErrors[index]} disabled={disabled} onChange={hole} />)}
      </fieldset>
      {validation && !validation.ok && <p className="course-form-error" role="alert" tabIndex={-1}>{validation.message}</p>}
      {error && <p className="course-form-error" role="alert">{error}</p>}
      <button className="course-save" type="submit" disabled={disabled}><Save aria-hidden="true" />{disabled ? 'Lagrer …' : 'Lagre manuell bane'}</button>
    </form>
  )
}

interface HoleRowProps {
  formId: string
  index: number
  value: ManualCourseDraft['holes'][number]
  errors: Partial<Record<HoleField, string>> | undefined
  disabled: boolean
  onChange: (index: number, field: HoleField, value: string) => void
}

function HoleRow({ formId, index, value, errors, disabled, onChange }: HoleRowProps) {
  const input = (field: HoleField, label: string, inputValue: string, placeholder?: string) => {
    const errorId = `${formId}-hole-${index}-${field}-error`
    return <label><span>{label}, hull {index + 1}</span><input inputMode="numeric" placeholder={placeholder} value={inputValue} aria-invalid={Boolean(errors?.[field])} aria-describedby={errors?.[field] ? errorId : undefined} data-validation-key={`holes.${index}.${field}`} onChange={(event) => onChange(index, field, event.target.value)} disabled={disabled} />{errors?.[field] && <small id={errorId} className="field-error">{errors[field]}</small>}</label>
  }
  return (
    <div className="manual-hole-row">
      <strong>{index + 1}</strong>
      {input('par', 'Par', value.par)}
      {input('strokeIndex', 'Slagindeks', value.strokeIndex)}
      {input('distance', 'Avstand i yards', value.distance, 'Valgfritt')}
    </div>
  )
}
