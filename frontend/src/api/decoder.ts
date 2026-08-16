const uuidPattern = /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i
const timestampPattern = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/
const datePattern = /^(\d{4})-(\d{2})-(\d{2})$/

export function invalidData(label: string, path: string): never {
  throw new Error(`Ugyldig ${label} fra serveren (${path})`)
}

export function decodeObject(value: unknown, path: string, label = 'data'): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) invalidData(label, path)
  return value as Record<string, unknown>
}

export function decodeString(value: unknown, path: string, label = 'data'): string {
  if (typeof value !== 'string') invalidData(label, path)
  return value
}

export function decodeUuid(value: unknown, path: string, label = 'data'): string {
  const decoded = decodeString(value, path, label)
  if (!uuidPattern.test(decoded)) invalidData(label, path)
  return decoded
}

export function isCanonicalUuid(value: string): boolean {
  return uuidPattern.test(value)
}

export function decodeTimestamp(value: unknown, path: string, label = 'data'): string {
  const decoded = decodeString(value, path, label)
  if (!timestampPattern.test(decoded)) invalidData(label, path)
  return decoded
}

export function decodeDate(value: unknown, path: string, label = 'data'): string {
  const decoded = decodeString(value, path, label)
  const parts = datePattern.exec(decoded)
  if (!parts) invalidData(label, path)
  const year = Number(parts[1])
  const month = Number(parts[2])
  const day = Number(parts[3])
  const date = new Date(Date.UTC(year, month - 1, day))
  if (date.getUTCFullYear() !== year || date.getUTCMonth() !== month - 1 || date.getUTCDate() !== day) {
    invalidData(label, path)
  }
  return decoded
}

export function decodeNumber(
  value: unknown,
  path: string,
  minimum?: number,
  maximum?: number,
  label = 'data',
): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) invalidData(label, path)
  if (minimum !== undefined && value < minimum) invalidData(label, path)
  if (maximum !== undefined && value > maximum) invalidData(label, path)
  return value
}

export function decodeInteger(
  value: unknown,
  path: string,
  minimum?: number,
  maximum?: number,
  label = 'data',
): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value)) invalidData(label, path)
  if (minimum !== undefined && value < minimum) invalidData(label, path)
  if (maximum !== undefined && value > maximum) invalidData(label, path)
  return value
}

export function decodeBoolean(value: unknown, path: string, label = 'data'): boolean {
  if (typeof value !== 'boolean') invalidData(label, path)
  return value
}

export function decodeArray<T>(
  value: unknown,
  path: string,
  decode: (item: unknown, path: string) => T,
  label = 'data',
): T[] {
  if (!Array.isArray(value)) invalidData(label, path)
  return value.map((item, index) => decode(item, `${path}[${index}]`))
}
