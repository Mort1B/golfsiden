use super::*;
use golf_api::domain::match_play::commands::Event;
use serde_json::Value;

pub(super) async fn assert_parity(pool: &PgPool, session: Uuid, player: Uuid) -> Value {
    let full =
        serde_json::to_value(match_play::reads::list(pool, session, id(5)).await.unwrap()).unwrap();
    assert!(full.get("player_id").is_none());
    let selected = serde_json::to_value(
        match_play::reads::list_for_player(pool, session, id(5), player)
            .await
            .unwrap(),
    )
    .unwrap();
    let cards: Vec<_> = full["matches"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|card| {
            card["opponents"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["player_id"] == player.to_string())
        })
        .cloned()
        .collect();
    let writable: Vec<_> = full["writable_match_ids"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|id| cards.iter().any(|card| card["match_id"] == **id))
        .cloned()
        .collect();
    assert!(cards.len() <= 1);
    assert_eq!(
        selected,
        serde_json::json!({"round_id":id(5),"player_id":player,"matches":cards,"writable_match_ids":writable})
    );
    selected
}

#[sqlx::test(migrations = "../migrations")]
async fn filtered_listing_preserves_roles_states_visibility_and_frozen_handicaps(pool: PgPool) {
    let (admin, first, second) = ready(&pool).await;
    let scorer = fixture::session(&pool, 201, None, "scorer", "listing-scorer").await;
    let player = fixture::session(&pool, 202, Some(11), "player", "listing-player").await;
    let viewer = fixture::session(&pool, 203, None, "viewer", "listing-viewer").await;
    sqlx::query(
        "INSERT INTO players(id,display_name,current_handicap_index) VALUES($1,'Nonparticipant',0)",
    )
    .bind(id(999))
    .execute(&pool)
    .await
    .unwrap();
    for state in ["open", "completed", "locked"] {
        if state == "completed" {
            for (m, conceder) in [(first, id(12)), (second, id(14))] {
                submit(
                    &pool,
                    admin,
                    m,
                    1,
                    Command::Report {
                        event: Event::Concession {
                            conceding_player_id: conceder,
                            communicated: true,
                            after_hole: 0,
                        },
                    },
                )
                .await;
                submit(
                    &pool,
                    admin,
                    m,
                    2,
                    Command::Confirm {
                        result_agreed_or_awarded: true,
                    },
                )
                .await;
            }
            golf_api::repositories::round_completion::complete_authorized(&pool, admin, id(5))
                .await
                .unwrap();
        } else if state == "locked" {
            golf_api::repositories::round_completion::lock_authorized(&pool, admin, id(5))
                .await
                .unwrap();
        }
        for hidden in [true, false] {
            let visibility =
                golf_api::repositories::tournament_visibility::get_for_admin(&pool, id(1), id(2))
                    .await
                    .unwrap();
            golf_api::repositories::tournament_visibility::update_authorized(
                &pool,
                admin,
                id(2),
                hidden,
                visibility.visibility_updated_at,
            )
            .await
            .unwrap();
            for session in [admin, scorer, player, viewer] {
                for target in [11, 12, 13, 14, 999, 1000] {
                    let value = assert_parity(&pool, session, id(target)).await;
                    if target >= 999 {
                        assert_eq!(value["matches"], serde_json::json!([]));
                    } else {
                        assert_eq!(value["matches"][0]["round_status"], state);
                        assert_eq!(
                            value["matches"][0]["visibility"]["mode"],
                            if hidden && session != admin {
                                "front_nine"
                            } else {
                                "full"
                            }
                        );
                        if hidden && session != admin {
                            assert!(value["matches"][0]["confirmed"].is_null());
                            if state != "open" {
                                assert_eq!(value["matches"][0]["finish"]["type"], "conceded");
                            }
                        }
                        if session == viewer || state == "locked" && session != admin {
                            assert_eq!(value["writable_match_ids"], serde_json::json!([]));
                        }
                    }
                }
            }
        }
    }
    let before = assert_parity(&pool, player, id(11)).await;
    sqlx::query("UPDATE players SET current_handicap_index=20 WHERE id=$1")
        .bind(id(11))
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(before, assert_parity(&pool, player, id(11)).await);
    sqlx::query("DELETE FROM tournament_memberships WHERE user_id=$1")
        .bind(id(202))
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        match_play::reads::list_for_player(&pool, player, id(5), id(11)).await,
        Err(match_play::Error::Forbidden)
    ));
    assert!(matches!(
        match_play::reads::list_for_player(&pool, admin, id(9999), id(11)).await,
        Err(match_play::Error::NotFound)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn draft_filtered_listing_is_not_writable_and_keeps_unassigned_empty(pool: PgPool) {
    let admin = fixture::draft(&pool, "singles_match_play").await;
    let updated = sqlx::query_scalar("SELECT updated_at FROM rounds WHERE id=$1")
        .bind(id(5))
        .fetch_one(&pool)
        .await
        .unwrap();
    match_play::setup::replace(
        &pool,
        admin,
        id(5),
        Assignments {
            expected_round_updated_at: updated,
            matches: vec![Pair {
                first_player_id: id(11),
                second_player_id: id(12),
            }],
        },
    )
    .await
    .unwrap();
    let viewer = fixture::session(&pool, 204, None, "viewer", "draft-listing-viewer").await;
    for session in [admin, viewer] {
        for target in [11, 12, 13] {
            let value = assert_parity(&pool, session, id(target)).await;
            assert_eq!(value["writable_match_ids"], serde_json::json!([]));
            if target == 13 {
                assert_eq!(value["matches"], serde_json::json!([]));
            } else {
                assert_eq!(value["matches"][0]["round_status"], "draft");
            }
        }
    }
}
