import { describe, expect, it } from 'vitest'
import { decodeFourBallInput, decodeFourBallRead, decodeFourBallScoring } from './decoders'
import { fourBallFixture, fourBallIds as ids } from './fixtures'
import { equalInput, expectedFourBall } from './contracts'
const decode = (value: unknown) => decodeFourBallScoring(value, ids.round, ids.team)
describe('four-ball side decoding', () => {
  it('accepts empty and complete sides with an unentered partner', () => {
    expect(decode(fourBallFixture()).gross_total).toBeNull()
    expect(decode(fourBallFixture(true))).toEqual(fourBallFixture(true))
  })
  it('distinguishes pickup from absence, retains identity and rejects open input variants', () => {
    const card = fourBallFixture(true)
    const hole = card.holes[0]
    if (!hole?.players[0].score) throw new Error('fixture')
    const score = hole.players[0].score
    expect(expectedFourBall({ ...score, input: { type: 'no_score' } })).toEqual({ type: 'present', score_id: score.id, revision: '1' })
    expect(equalInput({ type: 'no_score' }, { type: 'no_score' })).toBe(true)
    expect(equalInput(null, { type: 'no_score' })).toBe(false)
    expect(() => decodeFourBallInput({ type: 'no_score', gross_strokes: 4 })).toThrow()
    expect(() => decodeFourBallInput({ type: 'numeric', gross_strokes: 21 })).toThrow()
  })
  it('checks independent gross/net winner identities, including ties', () => {
    const card = fourBallFixture(true); const hole = card.holes[0]
    if (!hole?.players[0].score) throw new Error('fixture')
    hole.players[1].score = { ...hole.players[0].score, id: crypto.randomUUID(), owner: { type: 'player', id: ids.second } }
    hole.players[1].net_strokes = 3
    hole.gross = { strokes: 4, player_ids: [ids.first, ids.second] }
    hole.net = { strokes: 3, player_ids: [ids.second] }; card.net_total = 71
    expect(decode(card).holes[0]?.net?.player_ids).toEqual([ids.second])
    hole.net.player_ids = [ids.first]
    expect(() => decode(card)).toThrow()
  })
  it('rejects incorrect partner allocation, identities and scalar totals', () => {
    const card = fourBallFixture(true)
    expect(() => decode({ ...card, gross_total: 73 })).toThrow()
    expect(() => decode({ ...card, owner: { type: 'player', id: ids.team } })).toThrow()
    const hole = card.holes[0]; if (!hole) throw new Error('fixture')
    hole.players[1].handicap_strokes = 0
    expect(() => decode(card)).toThrow()
  })
  it('requires actor-free read entries and withholds all hidden completion metadata', () => {
    const card = fourBallFixture(true)
    expect(() => decodeFourBallRead(card, ids.round, ids.team)).toThrow()
    const read = { ...card, holes: card.holes.slice(0, 9).map(hole => ({ ...hole, players: hole.players.map(player => ({ ...player,
      score: player.score ? { id: player.score.id, input: player.score.input } : null })) })),
      visibility: { mode: 'front_nine' }, visible_hole_count: 9, complete: null, confirmed: null, confirmed_at: null,
      gross_total: 36, net_total: 36, par_played: 36, holes_scored: 9 }
    expect(decodeFourBallRead(read, ids.round, ids.team).visible_hole_count).toBe(9)
    expect(() => decodeFourBallRead({ ...read, confirmed: false }, ids.round, ids.team)).toThrow()
    expect(() => decodeFourBallRead({ ...read, holes: [...read.holes, card.holes[9]] }, ids.round, ids.team)).toThrow()
    expect(() => decode(read)).toThrow()
  })
})
