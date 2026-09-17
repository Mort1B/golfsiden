use super::*;
use golf_api::domain::{
    course_revisions::{
        self, CourseRevisionCommand, CourseRevisionSource, HoleRevisionCommand, TeeRevisionCommand,
    },
    models::{Round, ScoringFormat},
    tournament_plan::{RoundInput, TournamentPlanInput, normalize},
};
use golf_api::repositories::{course_presets, tournament_plan};

const RESTRICTED: [(ScoringFormat, &str, &str); 3] = [
    (
        ScoringFormat::FourBallStrokePlay,
        "four_ball_requires_18_holes",
        "four-ball requires 18 holes",
    ),
    (
        ScoringFormat::IndividualStableford,
        "stableford_requires_18_holes",
        "Stableford requires 18 holes",
    ),
    (
        ScoringFormat::SinglesMatchPlay,
        "singles_match_requires_18_holes",
        "singles match play requires 18 holes",
    ),
];
const LEGACY: [ScoringFormat; 3] = [
    ScoringFormat::IndividualStrokePlay,
    ScoringFormat::TeamScramble,
    ScoringFormat::TwoPlayerFoursomes,
];

async fn setup(pool: &PgPool) -> (Uuid, Vec<Round>) {
    sqlx::query("INSERT INTO users(id,username,display_name) VALUES($1,'layout_admin','Admin')")
        .bind(ADMIN)
        .execute(pool)
        .await
        .unwrap();
    let session = auth::create_session(
        pool,
        ADMIN,
        &hash_session_token(ADMIN_TOKEN),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    let date = (Utc::now() + Duration::days(1)).date_naive();
    let plan = normalize(TournamentPlanInput {
        tournament_name: "Course layout rules".into(),
        description: "".into(),
        start_date: date,
        end_date: date,
        counted_rounds: Some(1),
        mandatory_round_number: None,
        rounds: RESTRICTED
            .iter()
            .map(|(format, _, _)| *format)
            .chain(LEGACY)
            .enumerate()
            .map(|(index, scoring_format)| RoundInput {
                round_number: (index + 1) as i16,
                name: format!("Round {index}"),
                round_date: date,
                scoring_format,
            })
            .collect(),
    })
    .unwrap();
    let mut tx = pool.begin().await.unwrap();
    let (_, rounds) = tournament_plan::insert(&mut tx, ADMIN, None, &plan)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    (session.session_id, rounds)
}

fn selection(holes: i16) -> Value {
    json!({"source":"manual","course_name":"Layout course","location":null,"tee":{
        "category":"male","name":"Layout tee","course_rating":36.0,"slope_rating":113,
        "holes":(1..=holes).map(|stroke_index| json!({"par":4,"stroke_index":stroke_index,"distance":null})).collect::<Vec<_>>()
    }})
}

fn configure_request(round: &Round, selection: Value) -> Request<Body> {
    request(
        round.id,
        Some(ADMIN_TOKEN),
        true,
        json!({"expected_round_updated_at":round.updated_at,"selection":selection}),
    )
}

async fn snapshot(pool: &PgPool) -> Value {
    sqlx::query_scalar(
        "SELECT jsonb_build_object(
        'rounds',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM rounds r),
        'courses',(SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM courses c),
        'tees',(SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM tees t),
        'holes',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM holes h))",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn rejected(response: axum::response::Response, code: &str, message: &str) {
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert_eq!(
        response_json(response).await,
        json!({"error":{"code":code,"message":message}})
    );
}

async fn manual_rejection(pool: &PgPool, index: usize) {
    let (_, rounds) = setup(pool).await;
    let round = &rounds[index];
    let (_, code, message) = RESTRICTED[index];
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    // A nontransactional sequence proves rejection precedes even an attempted INSERT.
    sqlx::raw_sql("CREATE SEQUENCE layout_insert_probe;
        CREATE FUNCTION probe_layout_insert() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN PERFORM nextval('layout_insert_probe'); RETURN NEW; END $$;
        CREATE TRIGGER probe_layout_insert BEFORE INSERT ON courses FOR EACH ROW EXECUTE FUNCTION probe_layout_insert();")
        .execute(pool).await.unwrap();
    let before = snapshot(pool).await;
    rejected(
        app.clone()
            .oneshot(configure_request(round, selection(9)))
            .await
            .unwrap(),
        code,
        message,
    )
    .await;
    assert_eq!(snapshot(pool).await, before);
    assert_no_event(&mut events);
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT is_called FROM layout_insert_probe")
            .fetch_one(pool)
            .await
            .unwrap(),
        "layout must be checked before course INSERT"
    );
    let accepted = app
        .oneshot(configure_request(round, selection(18)))
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);
    assert_eq!(response_json(accepted).await["number_of_holes"], 18);
    assert_eq!(events.try_recv().unwrap().id, round.id);
    assert_no_event(&mut events);
}

#[sqlx::test(migrations = "../migrations")]
async fn four_ball_manual_nine_holes_rejected_before_insert(pool: PgPool) {
    manual_rejection(&pool, 0).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn stableford_manual_nine_holes_names_stableford(pool: PgPool) {
    manual_rejection(&pool, 1).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn singles_manual_nine_holes_names_match_play(pool: PgPool) {
    manual_rejection(&pool, 2).await;
}

fn manual_revision(holes: i16) -> course_revisions::ValidatedCourseRevision {
    course_revisions::validate(CourseRevisionCommand {
        source: CourseRevisionSource::Manual,
        provider_course_id: None,
        course_name: "Saved nine holes".into(),
        location: None,
        tee: TeeRevisionCommand {
            category: course_revisions::TeeCategory::Male,
            name: "Layout tee".into(),
            course_rating: 36.0,
            slope_rating: 113,
            holes: (1..=holes)
                .map(|stroke_index| HoleRevisionCommand {
                    par: 4,
                    stroke_index,
                    distance: None,
                })
                .collect(),
        },
    })
    .unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn saved_nine_hole_preset_uses_same_format_specific_manual_boundary(pool: PgPool) {
    let (_, rounds) = setup(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    let preset = course_revision_repository::insert_in_transaction(&mut tx, &manual_revision(9))
        .await
        .unwrap();
    sqlx::query("INSERT INTO course_presets(course_id,display_order) VALUES($1,99)")
        .bind(preset.course_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let presets = course_presets::list_for_admin(&pool, ADMIN, rounds[0].tournament_id)
        .await
        .unwrap();
    let mut saved =
        serde_json::to_value(presets.iter().find(|p| p.id == preset.course_id).unwrap()).unwrap();
    saved.as_object_mut().unwrap().remove("id");
    saved["source"] = json!("manual");
    for hole in saved["tee"]["holes"].as_array_mut().unwrap() {
        hole.as_object_mut().unwrap().remove("number");
    }
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let before = snapshot(&pool).await;
    for (round, (_, code, message)) in rounds.iter().zip(RESTRICTED) {
        rejected(
            app.clone()
                .oneshot(configure_request(round, saved.clone()))
                .await
                .unwrap(),
            code,
            message,
        )
        .await;
        assert_eq!(snapshot(&pool).await, before);
        assert_no_event(&mut events);
    }
}

fn provider_revision(holes: i16) -> course_revisions::ValidatedCourseRevision {
    select_and_validate(
        CourseDetail {
            provider: "golf_course_api",
            provider_course_id: "provider-fact-id".into(),
            club_name: "Provider club".into(),
            course_name: "Provider course".into(),
            scorecard_url: None,
            location: CourseLocation::default(),
            tees: vec![Tee {
                category: TeeCategory::Male,
                name: "Layout tee".into(),
                course_rating: 36.0,
                slope_rating: 113,
                total_yards: 0,
                total_meters: 0,
                number_of_holes: i32::from(holes),
                par_total: i32::from(holes) * 4,
                holes: (1..=holes)
                    .map(|number| Hole {
                        number: usize::try_from(number).unwrap(),
                        par: 4,
                        yardage: 400,
                        stroke_index: i32::from(number),
                    })
                    .collect(),
            }],
        },
        TeeCategory::Male,
        "Layout tee",
    )
    .unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn provider_tee_rejects_nine_and_accepts_eighteen_for_restricted_formats(pool: PgPool) {
    let (session, rounds) = setup(&pool).await;
    for round in rounds.iter().take(3) {
        let before = snapshot(&pool).await;
        let error = round_configuration::configure(
            &pool,
            session,
            round.id,
            round.updated_at,
            &provider_revision(9),
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            match round.scoring_format {
                ScoringFormat::FourBallStrokePlay => "four-ball requires 18 holes",
                ScoringFormat::IndividualStableford => "Stableford requires 18 holes",
                ScoringFormat::SinglesMatchPlay => "singles match play requires 18 holes",
                _ => unreachable!(),
            }
        );
        assert_eq!(snapshot(&pool).await, before);
        let accepted = round_configuration::configure(
            &pool,
            session,
            round.id,
            round.updated_at,
            &provider_revision(18),
        )
        .await
        .unwrap();
        assert_eq!(accepted.number_of_holes, 18);
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn legacy_formats_still_accept_nine_and_eighteen_holes(pool: PgPool) {
    let (_, rounds) = setup(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    for round in rounds.iter().skip(3) {
        for count in [9, 18] {
            let body = json!({"expected_round_updated_at":timestamp(&pool, round.id).await,"selection":selection(count)});
            let response = app
                .clone()
                .oneshot(request(round.id, Some(ADMIN_TOKEN), true, body))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response_json(response).await["number_of_holes"], count);
            assert_eq!(events.try_recv().unwrap().id, round.id);
            assert_no_event(&mut events);
        }
    }
}
