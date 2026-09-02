# Frontend Agent Rules

These rules apply to `frontend/` in addition to the repository contract.

## TypeScript mandates

- TypeScript strict mode stays enabled. New code must pass with
  `noUncheckedIndexedAccess`, `noUnusedLocals`, and `noUnusedParameters`.
- Do not use `any`, `@ts-ignore`, `@ts-nocheck`, or broad type assertions to
  silence errors. Use `unknown` at untrusted boundaries and narrow it explicitly.
- Non-null assertions are allowed only at a proven bootstrap boundary, such as
  the root DOM mount. Prefer guards everywhere else.
- Model finite backend states with string unions or discriminated unions. Keep
  API field names and nullability accurate; do not make required server fields
  optional for convenience.
- Validate or deliberately narrow untrusted runtime data. Compile-time types do
  not validate network responses.
- Prefer explicit component props and return types for exported utilities. Avoid
  opaque utility types that make public contracts harder to read.
- Dates remain ISO strings at the API boundary and are formatted in one focused
  utility. Do not rely on browser-dependent implicit date parsing.

## React structure and state

- Prefer components, hooks, and utilities in the 80-200 line range. Split before
  a component or hook exceeds roughly 250 lines, even though the repository hard
  limit is 400 lines.
- One component should own one user-facing responsibility. Extract data loading,
  mutation coordination, or complex derived state into focused hooks.
- Server state belongs in TanStack Query. Do not duplicate query data into local
  state or fetch directly inside components with ad hoc `useEffect` calls.
- Query keys must be stable, hierarchical, and include every resource identifier
  that affects the response. Mutations must invalidate or update the precise
  affected keys.
- Local state is for transient UI input and interaction. Derive values during
  render instead of synchronizing derived state through effects.
- Every effect needs a concrete external synchronization purpose and cleanup for
  subscriptions, timers, requests, or event listeners.
- Avoid prop drilling through more than two unrelated layers. Prefer composition
  first and narrowly scoped context only for genuinely shared application state.
- Do not create global stores for state already owned by the router, a form, or
  TanStack Query.
- Route modules own route composition; reusable UI does not read arbitrary route
  params unless routing is part of its explicit responsibility.

## API and failure handling

- All HTTP access goes through the typed API boundary under `src/api/`.
- Do not swallow errors. User-facing queries and mutations need deliberate
  loading, error, empty, success, and retry/sync states relevant to the flow.
- Prevent duplicate submissions and show whether score mutations are saving,
  synced, queued, or failed.
- SSE events are invalidation signals. Do not merge partial event payloads into
  authoritative score or leaderboard state unless the protocol is explicitly
  redesigned and tested.
- Keep optimistic updates reversible. Prefer server-confirmed state for locked
  rounds, handicap snapshots, and corrected scores.

## Accessible mobile-first UI

- Design from 320px upward. Verify primary flows at 320-390px and at a desktop
  width before completion.
- Interactive controls use semantic `button`, `a`, `input`, `select`, and form
  elements. Clickable `div` or `span` elements are prohibited.
- Icon-only controls need accessible names and tooltips when the icon is not
  universally understood. Decorative icons are hidden from assistive technology.
- Form fields need visible labels, useful validation messages, correct input
  modes, and keyboard behavior. Never rely on color alone to communicate state.
- Touch targets are at least 44x44 CSS pixels for primary scoring and navigation
  controls.
- Keep focus visible and logical. New dialogs, menus, and overlays must support
  keyboard use, focus management, Escape behavior, and focus restoration.
- Text must not overflow, overlap, or become obscured by fixed navigation at any
  supported viewport. Reserve stable dimensions for score controls, counters,
  tabs, and bottom navigation.
- Use the existing visual language and Lucide icons. Avoid nested cards,
  desktop-dashboard density, decorative gradients, or inaccessible low contrast.
- Do not add inline styles except for truly dynamic measured values. Split CSS by
  responsibility before the main stylesheet approaches 300 lines.

## Frontend tests and validation

- Test user-visible behavior, not component implementation details.
- Pure utilities use dedicated unit tests. Components and hooks use Vitest and
  React Testing Library.
- Cover mutation failure, stale/refetch behavior, long names, empty collections,
  and locked/disabled states when relevant.
- Use browser evidence for layout or interaction changes. Inspect console errors
  and failed network requests, not screenshots alone.
- Run `npm run test`, `npm run typecheck`, `npm run lint`, and `npm run build`
  before completion. Do not weaken the test, ESLint, or TypeScript configuration
  to make a change pass.
