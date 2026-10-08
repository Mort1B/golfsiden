use super::super::*;
use crate::domain::{
    fantasy::{
        SourceToken,
        projection::{Facts, OwnerFacts},
    },
    models::{RoundStatus, ScoringFormat, TournamentRole},
    score_visibility::{VisibilityFacts, VisibilityMetadata, VisibilityMode, visibility},
};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
#[derive(sqlx::FromRow)]
pub(super) struct Round {
    pub id: Uuid,
    pub round_number: i16,
    pub name: String,
    pub status: RoundStatus,
    pub scoring_format: ScoringFormat,
    pub number_of_holes: i16,
    pub locked_at: Option<DateTime<Utc>>,
}
#[derive(sqlx::FromRow)]
pub(super) struct Identity {
    pub id: Uuid,
    pub display_name: String,
}
#[derive(sqlx::FromRow)]
pub(super) struct Manager {
    pub id: Uuid,
    pub display_name: String,
    pub entered_at: DateTime<Utc>,
}
#[derive(sqlx::FromRow)]
pub(super) struct SelectionRow {
    pub round_id: Uuid,
    pub user_id: Uuid,
    pub state: String,
    pub picks: Option<Vec<Uuid>>,
    pub captain: Option<Uuid>,
    pub origin: Option<String>,
    pub source_round: Option<Uuid>,
}
#[derive(sqlx::FromRow)]
struct DispositionRow {
    round_id: Uuid,
    owner_kind: String,
    owner_id: Uuid,
    source_token: Vec<u8>,
    disposed: bool,
}
pub(super) struct Loaded {
    pub rounds: Vec<Round>,
    pub golfers: Vec<Identity>,
    pub managers: Vec<Manager>,
    pub selections: BTreeMap<(Uuid, Uuid), SelectionRow>,
    pub owners: BTreeMap<Uuid, Vec<OwnerFacts>>,
    pub visibility: BTreeMap<Uuid, VisibilityMetadata>,
}
pub(super) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    t: Uuid,
    caller: Uuid,
) -> Result<Loaded, Error> {
    let (role,round_count,hidden):(TournamentRole,i16,bool)=sqlx::query_as("SELECT m.role,t.number_of_rounds,t.final_round_back_nine_hidden FROM tournaments t JOIN tournament_memberships m ON m.tournament_id=t.id WHERE t.id=$1 AND m.user_id=$2").bind(t).bind(caller).fetch_one(&mut **tx).await?;
    let rounds:Vec<Round>=sqlx::query_as("SELECT r.id,r.round_number,r.name,r.status,r.scoring_format,r.number_of_holes,f.locked_at FROM rounds r LEFT JOIN fantasy_rounds f ON f.round_id=r.id WHERE r.tournament_id=$1 ORDER BY r.round_number,r.id").bind(t).fetch_all(&mut **tx).await?;
    let golfers=sqlx::query_as("SELECT p.id,p.display_name FROM tournament_players tp JOIN players p ON p.id=tp.player_id WHERE tp.tournament_id=$1 ORDER BY p.id").bind(t).fetch_all(&mut **tx).await?;
    let managers=sqlx::query_as("SELECT e.user_id AS id,u.display_name,e.entered_at FROM fantasy_entries e JOIN users u ON u.id=e.user_id WHERE e.tournament_id=$1 ORDER BY e.user_id").bind(t).fetch_all(&mut **tx).await?;
    let selected:Vec<SelectionRow>=sqlx::query_as("SELECT s.round_id,s.user_id,s.state,CASE WHEN l.id IS NULL THEN NULL ELSE ARRAY[l.first_player,l.second_player,l.third_player,l.fourth_player] END AS picks,l.captain,l.origin,l.source_round FROM fantasy_selections s LEFT JOIN fantasy_lineups l ON l.id=s.lineup_id WHERE s.tournament_id=$1 AND (s.locked_at IS NOT NULL OR s.user_id=$2) ORDER BY s.round_id,s.user_id").bind(t).bind(caller).fetch_all(&mut **tx).await?;
    let dispositions:Vec<DispositionRow>=sqlx::query_as("SELECT DISTINCT ON(round_id,player_id IS NULL,coalesce(player_id,team_id)) round_id,CASE WHEN player_id IS NULL THEN 'team' ELSE 'player' END AS owner_kind,coalesce(player_id,team_id) AS owner_id,source_token,disposed FROM fantasy_dispositions WHERE tournament_id=$1 ORDER BY round_id,player_id IS NULL,coalesce(player_id,team_id),created_at DESC,id DESC").bind(t).fetch_all(&mut **tx).await?;
    let dispositions: BTreeMap<_, _> = dispositions
        .into_iter()
        .map(|d| {
            (
                (d.round_id, d.owner_kind, d.owner_id),
                (d.source_token, d.disposed),
            )
        })
        .collect();
    let targets:Vec<(Uuid,String,Uuid)>=sqlx::query_as("SELECT h.round_id,'player'::text,h.player_id FROM round_handicap_snapshots h JOIN rounds r ON r.id=h.round_id WHERE r.tournament_id=$1 AND r.status<>'draft' AND r.scoring_format::text IN ('individual_stroke_play','individual_stableford','singles_match_play') UNION ALL SELECT teams.round_id,'team'::text,teams.id FROM teams JOIN rounds r ON r.id=teams.round_id WHERE r.tournament_id=$1 AND r.status<>'draft' AND r.scoring_format::text IN ('team_scramble','two_player_foursomes','four_ball_stroke_play') ORDER BY 1,2,3").bind(t).fetch_all(&mut **tx).await?;
    let visibility: BTreeMap<_, _> = rounds
        .iter()
        .map(|r| {
            (
                r.id,
                visibility(VisibilityFacts {
                    role,
                    is_final_round: r.round_number == round_count,
                    status: r.status,
                    number_of_holes: r.number_of_holes,
                    back_nine_hidden: hidden,
                }),
            )
        })
        .collect();
    let mut owners: BTreeMap<Uuid, Vec<OwnerFacts>> = BTreeMap::new();
    for row in sources::bulk_facts(tx, &targets).await? {
        let withheld = visibility
            .get(&row.round_id)
            .is_some_and(|v| v.mode == VisibilityMode::FrontNine);
        let source = if withheld {
            SourceToken([0; 32])
        } else {
            SourceToken(sources::hash_facts(&row.facts)?)
        };
        let non_finish = if withheld {
            None
        } else {
            dispositions
                .get(&(row.round_id, row.owner_kind, row.owner_id))
                .filter(|(_, disposed)| *disposed)
                .map(|(token, _)| {
                    token
                        .as_slice()
                        .try_into()
                        .map(SourceToken)
                        .map_err(|_| Error::Invalid)
                })
                .transpose()?
        };
        let facts: Facts = serde_json::from_value(row.facts).map_err(|_| Error::Invalid)?;
        owners.entry(row.round_id).or_default().push(OwnerFacts {
            facts,
            source,
            non_finish,
        });
    }
    Ok(Loaded {
        rounds,
        golfers,
        managers,
        selections: selected
            .into_iter()
            .map(|s| ((s.round_id, s.user_id), s))
            .collect(),
        owners,
        visibility,
    })
}
