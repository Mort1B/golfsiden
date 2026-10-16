import type { CompletionIssue } from '../../../api/scorecards'
export function FantasyInputWarning({ issues }: { issues: CompletionIssue[] }) {
  if (!issues.length) return null
  return <div className="scoring-notice" role="status">
    <p>Fantasy venter på partnerfelt. Sportslig bekreftelse med tomme felt avgjør ikke Fantasy. Runden kan ikke låses før feltene er avklart og scorekortet er bekreftet på nytt.</p>
    <ul>{issues.map((issue, index) => <li key={index}>{issue.message}</li>)}</ul>
    <p>Registrer faktisk score eller velg bevisst «Plukket opp» der spilleren ikke har score. Tomme felt endres ikke automatisk.</p>
  </div>
}
