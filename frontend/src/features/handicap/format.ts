export type HandicapParseResult =
  | { ok: true; value: number }
  | { ok: false; message: string }

const handicapPattern = /^-?(?:\d+|\d+[.,]\d|[.,]\d)$/

export function parseHandicap(value: string): HandicapParseResult {
  const normalized = value.trim()
  if (!handicapPattern.test(normalized)) {
    return { ok: false, message: 'Bruk et tall med maks én desimal, for eksempel 14,4.' }
  }

  const parsed = Number(normalized.replace(',', '.'))
  if (!Number.isFinite(parsed) || parsed < -10 || parsed > 54) {
    return { ok: false, message: 'Handicap må være mellom −10,0 og 54,0.' }
  }
  return { ok: true, value: parsed }
}

const handicapFormatter = new Intl.NumberFormat('nb-NO', {
  minimumFractionDigits: 1,
  maximumFractionDigits: 1,
  useGrouping: false,
})

export function formatHandicap(value: number): string {
  return handicapFormatter.format(value)
}
