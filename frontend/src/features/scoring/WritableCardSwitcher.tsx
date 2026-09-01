import type { OwnerCompletionProgress, ScoreOwner } from '../../api/scorecards'
import { ownerProgressLabel } from './selection'

interface WritableCardSwitcherProps {
  owners: OwnerCompletionProgress[]
  selectedOwner: ScoreOwner
  disabled: boolean
  onSelect: (owner: ScoreOwner) => void
  onPrefetch: (owner: ScoreOwner) => void
}

export function WritableCardSwitcher(props: WritableCardSwitcherProps) {
  if (props.owners.length < 2) return null

  return (
    <nav className="writable-card-switcher" aria-label="Bytt mellom scorekort du kan føre">
      <p>Dine scorekort</p>
      <div className="writable-card-switcher-track">
        {props.owners.map((owner) => {
          const selected = owner.owner.type === props.selectedOwner.type
            && owner.owner.id === props.selectedOwner.id
          return (
            <button
              key={`${owner.owner.type}-${owner.owner.id}`}
              type="button"
              disabled={props.disabled}
              aria-pressed={selected}
              onClick={() => props.onSelect(owner.owner)}
              onFocus={() => {
                if (!props.disabled) props.onPrefetch(owner.owner)
              }}
              onPointerEnter={() => {
                if (!props.disabled) props.onPrefetch(owner.owner)
              }}
            >
              <strong>{owner.owner_name}</strong>
              <span>{selected ? 'Valgt · ' : ''}{ownerProgressLabel(owner)}</span>
            </button>
          )
        })}
      </div>
    </nav>
  )
}
