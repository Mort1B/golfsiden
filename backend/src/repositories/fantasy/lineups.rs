use super::*;
use crate::domain::fantasy::{GolferId, selections::Lineup};
use chrono::{DateTime, Utc};

#[derive(sqlx::FromRow)]
struct SelectionRow {
    user_id: Uuid,
    state: String,
    locked_at: Option<DateTime<Utc>>,
    lineup_id: Option<Uuid>,
}

pub async fn save(
    pool: &PgPool,
    session: Uuid,
    t: Uuid,
    r: Uuid,
    input: &Save,
) -> Result<Receipt, Error> {
    let lineup = Lineup::new(
        &input
            .picks
            .iter()
            .copied()
            .map(GolferId)
            .collect::<Vec<_>>(),
        GolferId(input.captain),
    )
    .map_err(|_| Error::Invalid)?;
    if input.expected_revision < 0 {
        return Err(Error::Invalid);
    }
    let (mut tx, u) = begin(pool, session, t, false).await?;
    enabled(&mut tx, t).await?;
    config::window(&mut tx, t, r).await?;
    let replay: Option<Receipt> = sqlx::query_as(&format!(
        "SELECT {} FROM fantasy_lineups WHERE user_id=$1 AND request_id=$2",
        RECEIPT_COLUMNS
    ))
    .bind(u)
    .bind(input.request_id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(replay) = replay {
        if replay.round_id != r
            || replay.picks != input.picks
            || replay.captain != input.captain
            || replay.expected_revision != Some(input.expected_revision)
        {
            return Err(Error::Conflict);
        }
        recheck(&mut tx, session).await?;
        tx.commit().await?;
        return Ok(replay);
    }
    lifecycle::finalize(&mut tx, t).await?;
    let window = config::window(&mut tx, t, r).await?;
    if window.locked_at.is_some() {
        return Err(Error::Closed);
    }
    let entered: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM fantasy_entries WHERE tournament_id=$1 AND user_id=$2)",
    )
    .bind(t)
    .bind(u)
    .fetch_one(&mut *tx)
    .await?;
    if !entered {
        return Err(Error::Unavailable);
    }
    let old = lifecycle::current(&mut tx, r, u).await?;
    let revision = old.as_ref().map_or(0, |v| v.revision);
    if revision > input.expected_revision {
        // The immutable request lookup above found no receipt while holding the
        // same round locks as every save. This request cannot be accepted later
        // with its original expected revision; transient conflicts stay distinct.
        return Err(Error::RevisionConflict);
    }
    if revision < input.expected_revision {
        return Err(Error::Conflict);
    }
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    let eligible = lifecycle::eligible(&mut tx, t, now).await?;
    if !lineup.picks().iter().all(|p| eligible.contains(p)) {
        return Err(Error::Invalid);
    }
    let id = Uuid::new_v4();
    let mut values = lineup.picks().iter();
    let a = values.next().ok_or(Error::Invalid)?.0;
    let b = values.next().ok_or(Error::Invalid)?.0;
    let c = values.next().ok_or(Error::Invalid)?.0;
    let d = values.next().ok_or(Error::Invalid)?.0;
    sqlx::query("INSERT INTO fantasy_lineups(id,tournament_id,round_id,user_id,revision,first_player,second_player,third_player,fourth_player,captain,origin,request_id,expected_revision) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'submitted',$11,$12)").bind(id).bind(t).bind(r).bind(u).bind(revision.checked_add(1).ok_or(Error::Conflict)?).bind(a).bind(b).bind(c).bind(d).bind(input.captain).bind(input.request_id).bind(revision).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO fantasy_selections(round_id,tournament_id,user_id,lineup_id,state) VALUES($1,$2,$3,$4,'draft') ON CONFLICT(round_id,user_id) DO UPDATE SET lineup_id=excluded.lineup_id").bind(r).bind(t).bind(u).bind(id).execute(&mut *tx).await?;
    recheck(&mut tx, session).await?;
    let closed:bool=sqlx::query_scalar("SELECT opened_at IS NOT NULL OR COALESCE(deadline<=clock_timestamp(),false) FROM fantasy_rounds WHERE round_id=$1").bind(r).fetch_one(&mut *tx).await?;
    if closed {
        return Err(Error::Closed);
    }
    let receipt = lifecycle::receipt(&mut tx, id).await?;
    tx.commit().await?;
    Ok(receipt)
}
pub async fn read_round(
    pool: &PgPool,
    session: Uuid,
    t: Uuid,
    r: Uuid,
) -> Result<RoundView, Error> {
    let (mut tx, u) = begin(pool, session, t, false).await?;
    enabled(&mut tx, t).await?;
    let finalized = lifecycle::finalize(&mut tx, t).await?;
    let window = config::window(&mut tx, t, r).await?;
    let entered: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM fantasy_entries WHERE tournament_id=$1 AND user_id=$2)",
    )
    .bind(t)
    .bind(u)
    .fetch_one(&mut *tx)
    .await?;
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    let eligible_players: Vec<Uuid> =
        lifecycle::eligible(&mut tx, t, window.locked_at.unwrap_or(now))
            .await?
            .into_iter()
            .map(|p| p.0)
            .collect();
    let rows:Vec<SelectionRow>=sqlx::query_as("SELECT user_id,state,locked_at,lineup_id FROM fantasy_selections WHERE round_id=$1 AND (user_id=$2 OR ($3 AND state<>'not_participating')) ORDER BY user_id").bind(r).bind(u).bind(window.locked_at.is_some()).fetch_all(&mut *tx).await?;
    let mut selections = Vec::with_capacity(rows.len());
    let ids: Vec<Uuid> = rows.iter().filter_map(|row| row.lineup_id).collect();
    let receipts: Vec<Receipt> = sqlx::query_as(&format!(
        "SELECT {} FROM fantasy_lineups WHERE id=ANY($1) ORDER BY id",
        RECEIPT_COLUMNS
    ))
    .bind(&ids)
    .fetch_all(&mut *tx)
    .await?;
    let mut receipts: std::collections::HashMap<Uuid, Receipt> = receipts
        .into_iter()
        .map(|receipt| (receipt.id, receipt))
        .collect();
    for SelectionRow {
        user_id,
        mut state,
        locked_at,
        lineup_id,
    } in rows
    {
        let receipt = match lineup_id {
            Some(id) => Some(receipts.remove(&id).ok_or(Error::Invalid)?),
            None => None,
        };
        if state == "draft"
            && receipt
                .as_ref()
                .is_some_and(|r| r.picks.iter().any(|p| !eligible_players.contains(p)))
        {
            state = "invalid_draft".into();
        }
        selections.push(Selection {
            user_id,
            state,
            locked_at,
            receipt,
        });
    }
    let carry_forward_preview = if window.locked_at.is_none() && entered {
        lifecycle::previous(&mut tx, t, r, u, now).await?
    } else {
        None
    };
    recheck(&mut tx, session).await?;
    tx.commit().await?;
    let selection_availability = if window.locked_at.is_some() {
        SelectionAvailability::Closed
    } else if !entered {
        SelectionAvailability::NotEntered
    } else if eligible_players.len() < 4 {
        SelectionAvailability::InsufficientPlayers
    } else {
        SelectionAvailability::Open
    };
    let carry_forward_eligible = carry_forward_preview
        .as_ref()
        .is_some_and(|r| r.picks.iter().all(|p| eligible_players.contains(p)));
    Ok(RoundView {
        selection_availability,
        carry_forward_eligible,
        finalized,
        window,
        entered,
        eligible_players,
        selections,
        carry_forward_preview,
    })
}
