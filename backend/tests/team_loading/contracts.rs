use super::support::*;
use axum::http::{StatusCode, header};
use golf_api::repositories::teams;
use serde_json::{Value, json};
use sqlx::PgPool;

#[sqlx::test(migrations = "../migrations")]
async fn batch_preserves_all_fields_sql_order_empty_teams_and_round_isolation(pool: PgPool) {
    let (app, session, token) = setup(&pool).await;
    let empty = round(&pool, 1, 0, false).await;
    let selected = round(&pool, 2, 12, false).await;
    let sibling = round(&pool, 3, 1, false).await;
    let foreign = round(&pool, 4, 1, true).await;
    for target in [empty, selected, sibling, foreign] {
        let expected = original(&pool, target).await;
        assert_eq!(
            serde_json::to_value(teams::list(&pool, target).await.unwrap()).unwrap(),
            expected
        );
        if target != foreign {
            assert_eq!(
                serde_json::to_value(
                    teams::list_for_session(&pool, session, target)
                        .await
                        .unwrap()
                )
                .unwrap(),
                expected
            );
            assert_eq!(http(&app, target, &token).await, expected);
        }
    }
    assert_eq!(http(&app, empty, &token).await, json!([]));
    let value = http(&app, selected, &token).await;
    let teams = value.as_array().unwrap();
    assert_eq!(teams.len(), 12);
    assert_eq!(
        teams.iter().filter(|t| t["members"] == json!([])).count(),
        3
    );
    assert!(
        teams
            .iter()
            .all(|t| t["round_id"] == selected.to_string()
                && t["tournament_id"] == id(2).to_string())
    );
    for team in teams.iter().filter(|t| t["members"] != json!([])) {
        let members = team["members"].as_array().unwrap();
        assert_eq!(members.len(), 8);
        assert!(members[..3].iter().all(|m| m["display_order"] == 1));
        assert_eq!(members[3]["display_order"], 2);
        assert!(
            members[4..]
                .iter()
                .all(|m| m["display_order"] == Value::Null)
        );
        for name in ["b-order", "b-null"] {
            let ids: Vec<_> = members
                .iter()
                .filter(|m| m["display_name"] == name)
                .map(|m| m["player_id"].as_str().unwrap())
                .collect();
            assert_eq!(ids.len(), 2);
            assert!(ids[0] < ids[1]);
        }
    }
    let denied = response(&app, foreign, &token).await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(denied.headers()[header::CACHE_CONTROL], "private, no-store");
    assert_eq!(body(denied).await["error"]["code"], "forbidden");
}
