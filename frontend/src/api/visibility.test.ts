import { describe, expect, it } from 'vitest'
import { decodeScoreVisibility } from './visibility'

describe('score visibility', () => {
  it('decodes only the server projection mode', () => {
    expect(decodeScoreVisibility({ mode: 'front_nine' }, 'visibility', 'resultatdata'))
      .toEqual({ mode: 'front_nine' })
  })

  it('rejects an unknown mode', () => {
    expect(() => decodeScoreVisibility({ mode: 'back_nine' }, 'visibility', 'resultatdata'))
      .toThrow('visibility.mode')
  })
})
