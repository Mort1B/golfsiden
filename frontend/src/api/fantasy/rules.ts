import { invalidData } from '../decoder'
import type { Category, HoleResult, RulesVersion } from './types'

// Validate the declared category's award. Category selection, handicap allocation,
// placement and all aggregation remain authoritative server calculations.
export function validateHoleAward(hole: HoleResult, version: RulesVersion, path: string): void {
  const awards: Record<Category, number> = {
    ace: version === 1 ? 10 : 20,
    albatross_or_better: version === 1 ? 10 : 5,
    eagle: 3, birdie: 1, par: 0, bogey: -1, double_bogey: -2,
    triple_bogey: -3, quadruple_or_worse: -5, pickup: -5,
  }
  if (hole.category !== null && (hole.points.state !== 'settled' || hole.points.total !== awards[hole.category])) {
    invalidData('Fantasy-hullpoeng for regelversjonen', path)
  }
  if (hole.category === null && hole.points.state === 'settled') invalidData('Fantasy-hullkategori', path)
}
