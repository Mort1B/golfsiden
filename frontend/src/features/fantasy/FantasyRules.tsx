import type { RulesVersion } from '../../api/fantasy'

export function FantasyRules({version,newGame=false}:{version:RulesVersion;newGame?:boolean}){return <details className="fantasy-rules"><summary>Regler og poeng</summary>
  <p>{newGame?'Nye Fantasy-spill bruker regelversjon 2.':`Dette spillet bruker regelversjon ${version}.`}{version===1&&' De låste reglene beholdes for hele spillet.'}</p>
  <p>Velg fire spillere og én kaptein hver runde. Kapteinen får doble poeng, også minuspoeng. Ingen budsjett eller byttestraff. Alle runder teller sammenlagt.</p>
  <p>I lagrunder får begge partnerne samme lagpoeng. Du kan velge begge. Partnerne kan endres mellom rundene.</p>
  <p>{version===1?'Faktisk hole-in-one +10. Netto albatross eller bedre uten hole-in-one +10.':'Faktisk hole-in-one (ett registrert bruttoslag) +20. Netto albatross eller bedre uten hole-in-one +5.'} Netto eagle +3, birdie +1, par 0, bogey −1, dobbelbogey −2, trippelbogey −3, kvadrupelbogey eller verre og pickup −5. Én kategori per hull. Netto ett slag er ikke nødvendigvis en hole-in-one.</p>
  {version===2&&<p>En faktisk hole-in-one gir +20 én gang, også når nettoresultatet er albatross. Som kaptein gir hullet +40; en netto albatross uten hole-in-one gir +10. Velger du begge lagpartnerne ved en lag-ace, får du +40, eller +60 med én som kaptein. En ace er verdt fire netto albatrosser eller 20 birdier. Dette er en valgt poengbalanse, ikke en beregning av sannsynlighet.</p>}
  <p>Plassering 1–8 gir 10, 8, 6, 5, 4, 3, 2, 1 poeng. Delte plasseringer får samme poeng. Lag rangeres én gang før begge partnerne får poengene.</p>
  <p>Matchspill gir bare +3 for seier, +1 for uavgjort, −1 for tap. Ingen hull- eller plasseringspoeng. Ikke-fullførere beholder registrerte poeng uten straff for uspilt hull og uten plassering.</p>
  <p>Uten nytt gyldig lag gjenbrukes nærmeste tidligere låste firer og kaptein dersom alle fire fortsatt er valgbare. Ellers blir det ingen lag og 0 poeng. Laget må først meldes på; tidligere runder etterfylles ikke.</p>
  <p>«Avventer» viser et registrert delresultat. «Foreløpig» kan endres; «avgjort» kan fortsatt korrigeres av administrator. Skjulte resultater vises først når turneringen frigir dem.</p>
</details>}
