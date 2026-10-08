use super::*;
use crate::domain::fantasy::{
    GolferId, RoundId,
    selections::{self, DeadlineSelection, EarlierLockedLineup, Origin},
};
use chrono::{DateTime, Utc};
use std::collections::BTreeSet;

pub(super) async fn eligible(
    tx: &mut Transaction<'_, Postgres>,
    t: Uuid,
    at: DateTime<Utc>,
) -> Result<BTreeSet<GolferId>, Error> {
    let ids:Vec<Uuid>=sqlx::query_scalar("SELECT player_id FROM (SELECT DISTINCT ON(player_id) player_id,eligible FROM fantasy_eligibility_history WHERE tournament_id=$1 AND effective_at<=$2 ORDER BY player_id,effective_at DESC,id DESC) h WHERE eligible ORDER BY player_id").bind(t).bind(at).fetch_all(&mut **tx).await?;
    Ok(ids.into_iter().map(GolferId).collect())
}
pub(super) async fn receipt(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Receipt, Error> {
    Ok(sqlx::query_as(&format!(
        "SELECT {} FROM fantasy_lineups WHERE id=$1",
        RECEIPT_COLUMNS
    ))
    .bind(id)
    .fetch_one(&mut **tx)
    .await?)
}
pub(super) async fn current(
    tx: &mut Transaction<'_, Postgres>,
    r: Uuid,
    u: Uuid,
) -> Result<Option<Receipt>, Error> {
    let id:Option<Uuid>=sqlx::query_scalar("SELECT lineup_id FROM fantasy_selections WHERE round_id=$1 AND user_id=$2 AND lineup_id IS NOT NULL").bind(r).bind(u).fetch_optional(&mut **tx).await?;
    match id {
        Some(id) => Ok(Some(receipt(tx, id).await?)),
        None => Ok(None),
    }
}
pub(super) async fn previous(
    tx: &mut Transaction<'_, Postgres>,
    t: Uuid,
    r: Uuid,
    u: Uuid,
    at: DateTime<Utc>,
) -> Result<Option<Receipt>, Error> {
    let id:Option<Uuid>=sqlx::query_scalar("SELECT s.lineup_id FROM fantasy_selections s JOIN rounds prior ON prior.id=s.round_id JOIN rounds target ON target.id=$2 WHERE s.tournament_id=$1 AND s.user_id=$3 AND s.state='locked' AND s.locked_at<=$4 AND prior.round_number<target.round_number ORDER BY prior.round_number DESC LIMIT 1").bind(t).bind(r).bind(u).bind(at).fetch_optional(&mut **tx).await?;
    match id {
        Some(id) => Ok(Some(receipt(tx, id).await?)),
        None => Ok(None),
    }
}
// Chronological effective close first, round order second. A future reader cannot
// make a source that closed later eligible for an earlier destination deadline.
pub(super) async fn finalize(tx: &mut Transaction<'_, Postgres>, t: Uuid) -> Result<bool, Error> {
    let due:Vec<(Uuid,i16,DateTime<Utc>)>=sqlx::query_as("SELECT f.round_id,r.round_number,least(f.deadline,f.opened_at) FROM fantasy_rounds f JOIN rounds r ON r.id=f.round_id WHERE f.tournament_id=$1 AND f.locked_at IS NULL AND least(f.deadline,f.opened_at)<=clock_timestamp() ORDER BY least(f.deadline,f.opened_at),r.round_number").bind(t).fetch_all(&mut **tx).await?;
    let changed = !due.is_empty();
    for (r, order, at) in due {
        let eligibility = eligible(tx, t, at).await?;
        let entrants:Vec<(Uuid,bool)>=sqlx::query_as("SELECT e.user_id,COALESCE((SELECT present FROM fantasy_membership_history h WHERE h.tournament_id=e.tournament_id AND h.user_id=e.user_id AND effective_at<=$2 ORDER BY effective_at DESC,id DESC LIMIT 1),false) FROM fantasy_entries e WHERE e.tournament_id=$1 AND e.entered_at<=$2 ORDER BY e.user_id").bind(t).bind(at).fetch_all(&mut **tx).await?;
        for (u, participating) in entrants {
            let accepted = current(tx, r, u).await?;
            let accepted_lineup = accepted.as_ref().map(Receipt::lineup).transpose()?;
            let prior = previous(tx, t, r, u, at).await?;
            let earlier = if let Some(prior) = &prior {
                let n: i16 = sqlx::query_scalar("SELECT round_number FROM rounds WHERE id=$1")
                    .bind(prior.round_id)
                    .fetch_one(&mut **tx)
                    .await?;
                vec![EarlierLockedLineup {
                    round: RoundId(prior.round_id),
                    round_order: n as u32,
                    lineup: prior.lineup()?,
                }]
            } else {
                vec![]
            };
            let resolved = selections::resolve_selection(DeadlineSelection {
                round: RoundId(r),
                round_order: order as u32,
                participating_at_deadline: participating,
                eligible_at_deadline: &eligibility,
                accepted_current: accepted_lineup.as_ref(),
                earlier_locked_at_deadline: &earlier,
            })
            .map_err(|_| Error::Invalid)?;
            let (state, id) = match resolved {
                selections::Selection::Locked {
                    lineup,
                    origin: Origin::Submitted,
                    ..
                } => (
                    "locked",
                    accepted
                        .as_ref()
                        .map(|a| a.id)
                        .filter(|_| lineup.picks().len() == 4),
                ),
                selections::Selection::Locked {
                    lineup,
                    origin: Origin::CarriedForward { source_round },
                    ..
                } => {
                    let revision = accepted
                        .as_ref()
                        .map_or(Ok(1), |a| a.revision.checked_add(1).ok_or(Error::Conflict))?;
                    let id = Uuid::new_v4();
                    let picks = lineup.picks();
                    sqlx::query("INSERT INTO fantasy_lineups(id,tournament_id,round_id,user_id,revision,first_player,second_player,third_player,fourth_player,captain,origin,source_round,accepted_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'carried_forward',$11,$12)")
      .bind(id).bind(t).bind(r).bind(u).bind(revision).bind(picks.first().ok_or(Error::Invalid)?.0).bind(picks.get(1).ok_or(Error::Invalid)?.0).bind(picks.get(2).ok_or(Error::Invalid)?.0).bind(picks.get(3).ok_or(Error::Invalid)?.0).bind(lineup.captain().0).bind(source_round.0).bind(at).execute(&mut **tx).await?;
                    ("locked", Some(id))
                }
                selections::Selection::Missed => ("missed", None),
                selections::Selection::Invalid => ("invalid", None),
                selections::Selection::NotParticipating => ("not_participating", None),
            };
            sqlx::query("INSERT INTO fantasy_selections(round_id,tournament_id,user_id,lineup_id,state,locked_at) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(round_id,user_id) DO UPDATE SET lineup_id=excluded.lineup_id,state=excluded.state,locked_at=excluded.locked_at")
    .bind(r).bind(t).bind(u).bind(id).bind(state).bind(at).execute(&mut **tx).await?;
        }
        sqlx::query("UPDATE fantasy_rounds SET locked_at=$2 WHERE round_id=$1")
            .bind(r)
            .bind(at)
            .execute(&mut **tx)
            .await?;
    }
    Ok(changed)
}
