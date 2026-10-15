import { expect, request } from '../../../frontend/node_modules/@playwright/test/index.mjs';
export const root = 'http://127.0.0.1:5173';
export async function ok(response, status) {
  expect(status === undefined ? response.ok() : response.status() === status).toBe(true);
  return response;
}
export async function fixture(page, n) {
  const day = new Date(Date.now() + 86400000).toISOString().slice(0, 10);
  const names = Array.from({ length: 8 }, (_, i) => `Synthetic golfer ${i + 1}`);
  const created = await ok(await page.request.post('/api/onboarding/tournaments', { data: {
    creator: { account: { username: `a1u_${crypto.randomUUID().replaceAll('-', '').slice(0, 20)}`, password: crypto.randomUUID() },
      player: { display_name: names[0], handicap_index: 0 } },
    tournament: { name: 'A1t synthetic fixture', description: '', start_date: day, end_date: day,
      counted_rounds: 1, mandatory_round_number: null },
    rounds: Array.from({ length: n }, (_, i) => ({ round_number: i + 1, name: `Round ${i + 1}`,
      round_date: day, scoring_format: 'team_scramble' })),
  } }), 201);
  const { session: auth, tournament } = await created.json();
  const mutate = async (path, data = {}, method = 'POST') => ok(await page.request.fetch(path,
    { method, data, headers: { 'x-csrf-token': auth.csrf_token } }));
  const players = [auth.player_id];
  for (let i = 1; i < 8; i++) players.push((await (await mutate(`/api/tournaments/${tournament.id}/players`,
    { display_name: names[i], handicap_index: 0 })).json()).player_id);
  const rounds = await (await ok(await page.request.get(`/api/tournaments/${tournament.id}/rounds`))).json();
  const base = `/api/tournaments/${tournament.id}/fantasy`;
  await mutate(base, { enabled: true }, 'PUT');
  await mutate(`${base}/entry`);
  const teams = [];
  for (const r of rounds) {
    const configured = await (await mutate(`/api/rounds/${r.id}/course-configuration`, {
      expected_round_updated_at: r.updated_at, selection: { source: 'manual', course_name: 'Synthetic course',
        location: null, tee: { category: 'male', name: 'Synthetic tee', course_rating: 72, slope_rating: 113,
          holes: Array.from({ length: 18 }, (_, i) => ({ par: 4, stroke_index: i + 1, distance: null })) } },
    }, 'PUT')).json();
    const pairs = Array.from({ length: 4 }, (_, i) => ({ id: crypto.randomUUID(), name: `Team ${i + 1}`,
      members: players.slice(i * 2, i * 2 + 2).map(player_id => ({ player_id })), schedule_flight_id: null }));
    await mutate(`/api/rounds/${r.id}/pairings`, { expected_round_updated_at: configured.updated_at, teams: pairs,
      flights: [0, 1].map(i => ({ id: crypto.randomUUID(), name: `Flight ${i + 1}`, starting_hole: 1, tee_time: null,
        members: players.slice(i * 4, i * 4 + 4).map(player_id => ({ player_id })) })), legacy_conversions: [] }, 'PUT');
    teams.push(pairs);
  }
  const claim = await (await mutate(`/api/tournaments/${tournament.id}/players/${players[1]}/claim`)).json();
  const other = await request.newContext({ baseURL: root });
  const registered = await (await ok(await other.post(`/api/player-claims/${claim.claim_id}/register`, { data: {
    token: claim.token, account: { username: `a1u_${crypto.randomUUID().replaceAll('-', '').slice(0, 20)}`, password: crypto.randomUUID() },
  } }), 201)).json();
  await ok(await other.post(`${base}/entry`, { headers: { 'x-csrf-token': registered.session.csrf_token } }), 204);
  await ok(await other.put(`${base}/rounds/${rounds[0].id}/lineup`, { headers: { 'x-csrf-token': registered.session.csrf_token },
    data: { request_id: crypto.randomUUID(), expected_revision: 0, picks: players.slice(4, 8), captain: players[5] } }));
  return { page, auth, tournament, names, players, rounds, teams, base, mutate, other,
    path: `${base}/rounds/${rounds[0].id}/results`, url: `/tournaments/${tournament.id}/fantasy` };
}
export async function settle(f) {
  const fresh = await (await ok(await f.page.request.get(`/api/tournaments/${f.tournament.id}`))).json();
  await f.mutate(`/api/tournaments/${f.tournament.id}/start`, { expected_tournament_updated_at: fresh.updated_at });
  for (const [i, r] of f.rounds.entries()) {
    await f.mutate(`/api/rounds/${r.id}/open`);
    for (const [j, team] of f.teams[i].entries()) {
      const path = `/api/rounds/${r.id}/scorecards/team/${team.id}`;
      const card = await (await ok(await f.page.request.get(`${path}/scoring`))).json();
      for (const [h, hole] of card.holes.entries()) await f.mutate(`/api/rounds/${r.id}/scores`,
        { owner: { type: 'team', id: team.id }, hole_id: hole.hole_id, gross_strokes: h === 0 ? 4 + j : 4 }, 'PUT');
      await f.mutate(`${path}/confirm`);
    }
    await f.mutate(`/api/rounds/${r.id}/complete`);
  }
}
