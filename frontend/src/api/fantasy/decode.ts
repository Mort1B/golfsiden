import { decodeArray, decodeInteger, decodeString, decodeTimestamp, decodeUuid, invalidData } from '../decoder'
export { decodeObject as object, decodeBoolean as boolean, decodeString as string, decodeUuid as uuid, decodeInteger as integer } from '../decoder'
export function nullable<T>(v: unknown, p: string, decode: (v: unknown, p: string) => T): T | null { return v === null ? null : decode(v, p) }
export function choice<const T extends readonly string[]>(v: unknown, p: string, options: T): T[number] {
  for (const option of options) if (v === option) return option
  return invalidData('Fantasy-data', p)
}
export function timestamp(v: unknown, p: string): string {
  const result = decodeTimestamp(v, p)
  if (!Number.isFinite(Date.parse(result))) invalidData('Fantasy-data', p)
  return result
}
export function array<T>(v: unknown, p: string, decode: (v: unknown, p: string) => T): T[] { return decodeArray(v, p, decode) }
export function ids(v: unknown, p: string): string[] { const result = array(v, p, decodeUuid); unique(result, p); return result }
export function unique(ids: string[], p: string): void { if (new Set(ids).size !== ids.length) invalidData('Fantasy-data', p) }
export function expected(v: unknown, p: string, id: string): string { const result = decodeUuid(v, p); if (result !== id) invalidData('Fantasy-data', p); return result }
export function rank(v: unknown, p: string): number | null { return nullable(v, p, (value, path) => decodeInteger(value, path, 1)) }
export function version(v: unknown, p: string): number { const result = decodeInteger(v, p); if (result !== 1) invalidData('Fantasy-regelversjon', p); return result }
export function token(v: unknown, p: string): string { const result = decodeString(v, p); if (!/^[A-Za-z0-9_-]{43}$/.test(result)) invalidData('Fantasy-revisjon', p); return result }
