import type { NetAwards } from '../../api/roundAwards/types'
import { awardLabels } from '../../api/roundAwards/types'
export function NetAwardSummary({awards}:{awards:NetAwards|undefined}){
 if(!awards)return null
 return <p className="net-awards">{awards.type==='stroke'?`Netto før utmerkelser: ${awards.base_net_total}. `:`Native nettopoeng: ${awards.base_net_points}. `}{awards.awards.map(kind=>`${awardLabels[kind]} −1`).join(' · ')}. {awards.type==='stroke'?`Justert netto: ${awards.adjusted_net_total}.`:`Utmerkelsespoeng: ${-awards.adjustment}. Konkurransepoeng: ${awards.competition_net_points}. Ekvivalent: ${awards.base_net_equivalent} → ${awards.adjusted_net_equivalent}.`}</p>
}
