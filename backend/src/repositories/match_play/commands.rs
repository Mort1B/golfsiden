use super::*;
use crate::domain::{
    match_play::{
        ConfirmationStatus,
        commands::{Acknowledgement, Basis, Command, Event, Request, reason_valid},
    },
    scorecards::ScoreRevision,
};
use crate::repositories::scorecards::MutationResult;
use sha2::{Digest, Sha256};
#[derive(sqlx::FromRow)]
struct Receipt {
    request_hash: Vec<u8>,
    applied_revision: i64,
    match_id: Uuid,
}
pub async fn execute(
    pool: &PgPool,
    session: Uuid,
    round: Uuid,
    id: Uuid,
    request: Request,
) -> Result<MutationResult<Acknowledgement>, Error> {
    let hash = Sha256::digest(
        serde_json::to_vec(&(round, id, &request))
            .map_err(|_| Error::Invalid("invalid command"))?,
    )
    .to_vec();
    let mut tx = pool.begin().await?;
    let c = context(&mut tx, round, true).await?;
    let m = aggregate(&mut tx, round, id, true).await?;
    let (actor, admin) = authority(&mut tx, session, &c, &m).await?;
    if let Some(receipt)=sqlx::query_as::<_,Receipt>("SELECT request_hash,applied_revision,match_id FROM singles_match_receipts WHERE user_id=$1 AND request_id=$2").bind(actor).bind(request.request_id).fetch_optional(&mut *tx).await? {
  if receipt.request_hash!=hash || receipt.match_id!=id {return Err(Error::Conflict("match_request_mismatch"));}
  live(&mut tx,session).await?;tx.commit().await?;
  return Ok(MutationResult{value:ack(request.request_id,id,receipt.applied_revision)?,changed:false,tournament_id:c.tournament_id});
 }
    if request.expected_revision.as_i64() != m.revision {
        return Err(Error::Conflict("match_revision_conflict"));
    }
    let correction = matches!(request.command, Command::Correct { .. });
    let confirmation = matches!(request.command, Command::Confirm { .. });
    if c.status == RoundStatus::Draft
        || (c.status == RoundStatus::Locked
            && !(admin && (correction || (confirmation && m.correction_pending))))
    {
        return Err(Error::Conflict("match_round_not_editable"));
    }
    if m.terminal && !correction && !confirmation {
        return Err(Error::Conflict("match_terminal"));
    }
    let (allocation, indexes) = allocation(&mut tx, &c, &m).await?;
    let mut ledger = m.events()?;
    let mut confirmed = m.confirmed;
    let mut pending = m.correction_pending;
    write_context(&mut tx, session, actor, round).await?;
    let command_json =
        serde_json::to_value(&request.command).map_err(|_| Error::Invalid("invalid command"))?;
    sqlx::query("SELECT set_config('app.match_command',$1,true),set_config('app.match_request',$2::text,true)").bind(command_json.to_string()).bind(request.request_id).execute(&mut *tx).await?;
    match &request.command {
        Command::Note {
            player_id,
            hole_number,
            gross_strokes,
        } => {
            if !(1..=20).contains(gross_strokes) {
                return Err(Error::Invalid("gross_strokes must be between 1 and 20"));
            }
            note(&mut tx, &m, *player_id, *hole_number, Some(*gross_strokes)).await?;
        }
        Command::ClearNote {
            player_id,
            hole_number,
        } => {
            note(&mut tx, &m, *player_id, *hole_number, None).await?;
        }
        Command::Report { event } => {
            if !admin && organizer(event) {
                return Err(Error::Forbidden);
            }
            ledger.push(AcceptedEvent {
                id: Uuid::new_v4(),
                event: event.clone(),
            });
        }
        Command::Confirm {
            result_agreed_or_awarded,
        } => {
            if !result_agreed_or_awarded {
                return Err(Error::Invalid(
                    "result agreement or award attestation is required",
                ));
            }
            confirmed = true;
            pending = false;
        }
        Command::Correct {
            reason,
            superseded_event_ids,
            replacement,
            ..
        } => {
            if !admin {
                return Err(Error::Forbidden);
            }
            if !reason_valid(reason) {
                return Err(Error::Invalid(
                    "correction requires a reason of 1 to 1000 bytes",
                ));
            }
            let actual = ledger.iter().map(|e| e.id).collect::<Vec<_>>();
            if actual != *superseded_event_ids {
                return Err(Error::Conflict("match_correction_ledger_conflict"));
            }
            ledger = replacement
                .iter()
                .map(|event| AcceptedEvent {
                    id: Uuid::new_v4(),
                    event: event.clone(),
                })
                .collect();
            confirmed = false;
            pending = true;
        }
    }
    // Existing accepted rulings retain their original authority; only newly supplied
    // rulings are authorized above. Derivation does not reauthorize historical facts.
    let state = validate_ledger(&ledger, m.players(), allocation, &indexes, true)
        .map_err(Error::Invalid)?;
    let points = state
        .point_award(if confirmed {
            ConfirmationStatus::Confirmed
        } else {
            ConfirmationStatus::Unconfirmed
        })
        .map_err(|_| Error::Conflict("match_not_terminal"))?;
    let revision = m
        .revision
        .checked_add(1)
        .ok_or(Error::Conflict("match_revision_exhausted"))?;
    let json = serde_json::to_value(&ledger).map_err(|_| Error::Invalid("invalid ledger"))?;
    sqlx::query("UPDATE singles_matches SET revision=$2,ledger=$3,terminal=$4,confirmed=$5,correction_pending=$6,first_half_points=$7,second_half_points=$8 WHERE id=$1").bind(id).bind(revision).bind(json).bind(state.finish().is_some()).bind(confirmed).bind(pending).bind(points.map(|p|p.first.half_units() as i16)).bind(points.map(|p|p.second.half_units() as i16)).execute(&mut *tx).await?;
    let inserted=sqlx::query("INSERT INTO singles_match_receipts(user_id,request_id,match_id,request_hash,applied_revision) VALUES($1,$2,$3,$4,$5) ON CONFLICT(user_id,request_id) DO NOTHING").bind(actor).bind(request.request_id).bind(id).bind(hash).bind(revision).execute(&mut *tx).await?.rows_affected();
    if inserted != 1 {
        return Err(Error::Conflict("match_request_mismatch"));
    }
    live(&mut tx, session).await?;
    tx.commit().await?;
    Ok(MutationResult {
        value: ack(request.request_id, id, revision)?,
        changed: true,
        tournament_id: c.tournament_id,
    })
}
fn ack(request_id: Uuid, match_id: Uuid, revision: i64) -> Result<Acknowledgement, Error> {
    Ok(Acknowledgement {
        request_id,
        match_id,
        applied_revision: ScoreRevision::from_database(revision)
            .ok_or(Error::Conflict("invalid match revision"))?,
    })
}
fn organizer(event: &Event) -> bool {
    matches!(
        event,
        Event::Award { .. }
            | Event::Hole {
                basis: Basis::OrganizerRuling { .. },
                ..
            }
    )
}
async fn note(
    tx: &mut Transaction<'_, Postgres>,
    m: &Aggregate,
    player: Uuid,
    hole: u8,
    gross: Option<i16>,
) -> Result<(), Error> {
    if !m.players().contains(&player) || !(1..=18).contains(&hole) {
        return Err(Error::Invalid(
            "note must identify an opponent and hole 1 to 18",
        ));
    }
    sqlx::query("SELECT set_config('app.match_note_id',$1::text,true)")
        .bind(m.id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO singles_match_notes(match_id,player_id,hole_number,gross_strokes) VALUES($1,$2,$3,$4) ON CONFLICT(match_id,player_id,hole_number) DO UPDATE SET gross_strokes=EXCLUDED.gross_strokes").bind(m.id).bind(player).bind(i16::from(hole)).bind(gross).execute(&mut **tx).await?;
    Ok(())
}
