use crate::{factories, support::TestApp};
use axum::http::StatusCode;
use board::{
    error::AppError,
    models::{Page, application, request},
    routes::paths,
};

async fn apply(
    app: &TestApp,
    person: &board::models::User,
    task: &request::Model,
) -> application::Model {
    application::create(
        app.db(),
        person,
        application::ApplicationParams {
            request_id: task.id.to_string(),
            pitch: "I can visit tomorrow and report what I find.".into(),
            contact: format!("private-{}@example.com", person.id),
        }
        .validate()
        .unwrap(),
    )
    .await
    .unwrap()
}

/// AC-board-1
#[tokio::test]
async fn public_board_searches_and_filters() {
    let app = TestApp::new().await;
    let owner = factories::user(app.db(), "owner@example.com").await;
    let mut p = factories::request_params(app.db(), &owner).await;
    p.title = "Visit a museum entrance".into();
    p.description = "Measure access in Berlin".into();
    p.category = "real-world".into();
    p.location = "Berlin".into();
    let task = factories::request_with(app.db(), &owner, p).await;
    app.get("/?q=MUSEUM&location=berlin&category=real-world")
        .await
        .assert_ok()
        .assert_text(&task.title);
    app.get("/?category=testing")
        .await
        .assert_ok()
        .assert_not_contains(&task.title);
    app.get("/?q=%25")
        .await
        .assert_ok()
        .assert_not_contains(&task.title);
    app.get(&paths::task(task.id))
        .await
        .assert_ok()
        .assert_text(&task.description);
}

/// AC-board-2
#[tokio::test]
async fn cannot_post_for_another_users_agent() {
    let app = TestApp::new().await;
    let owner = factories::user(app.db(), "owner@example.com").await;
    let other = factories::user(app.db(), "other@example.com").await;
    let params = factories::request_params(app.db(), &owner).await;
    assert!(matches!(
        request::create(app.db(), &other, params.validate().unwrap()).await,
        Err(AppError::Invalid(_))
    ));
}

/// AC-board-2
#[tokio::test]
async fn invalid_board_inputs_are_rejected() {
    let app = TestApp::new().await;
    let owner = factories::user(app.db(), "owner@example.com").await;
    let mut p = factories::request_params(app.db(), &owner).await;
    p.budget = "-1.00".into();
    assert!(p.validate().is_err());
    p.budget = "20.00".into();
    p.category = "arbitrary".into();
    assert!(p.validate().is_err());
    let filter = request::BoardFilter {
        q: "x".repeat(201),
        ..Default::default()
    };
    assert!(
        request::browse(app.db(), &filter, Page::default())
            .await
            .is_err()
    );
    let filter = request::BoardFilter::default();
    let (rows, more) = request::browse(app.db(), &filter, Page::new(u64::MAX, u64::MAX))
        .await
        .unwrap();
    assert!(rows.is_empty());
    assert!(!more);
}

/// AC-board-3
#[tokio::test]
async fn anonymous_cannot_apply() {
    let app = TestApp::new().await;
    app.post_form(
        "/tasks/1/applications",
        &[("pitch", "I can help"), ("contact", "private@example.com")],
    )
    .await
    .assert_redirect_to(&paths::new_session());
}

/// AC-board-3
#[tokio::test]
async fn applications_are_once_per_person_and_only_while_open() {
    let app = TestApp::new().await;
    let owner = factories::user(app.db(), "owner@example.com").await;
    let human = factories::user(app.db(), "human@example.com").await;
    let task = factories::request(app.db(), &owner).await;
    let params = application::ApplicationParams {
        request_id: task.id.to_string(),
        pitch: "I can help".into(),
        contact: "private@example.com".into(),
    };
    assert!(
        application::create(app.db(), &owner, params.validate().unwrap())
            .await
            .is_err()
    );
    apply(&app, &human, &task).await;
    assert!(
        application::create(app.db(), &human, params.validate().unwrap())
            .await
            .is_err()
    );
    request::close_for(app.db(), &owner, task.id).await.unwrap();
    let another = factories::user(app.db(), "another@example.com").await;
    assert!(
        application::create(app.db(), &another, params.validate().unwrap())
            .await
            .is_err()
    );
}

/// AC-board-4
#[tokio::test]
async fn owner_selects_one_applicant() {
    let app = TestApp::new().await;
    let owner = factories::user(app.db(), "owner@example.com").await;
    let alice = factories::user(app.db(), "alice@example.com").await;
    let bob = factories::user(app.db(), "bob@example.com").await;
    let task = factories::request(app.db(), &owner).await;
    let first = apply(&app, &alice, &task).await;
    let second = apply(&app, &bob, &task).await;
    assert!(
        application::select_for(app.db(), &owner, first.id)
            .await
            .unwrap()
            .selected_at
            .is_some()
    );
    assert!(
        application::select_for(app.db(), &owner, second.id)
            .await
            .is_err()
    );
    assert_eq!(
        request::find_for(app.db(), &owner, task.id)
            .await
            .unwrap()
            .status,
        request::RequestStatus::Assigned
    );
    request::close_for(app.db(), &owner, task.id).await.unwrap();
    assert_eq!(
        request::find_for(app.db(), &owner, task.id)
            .await
            .unwrap()
            .status,
        request::RequestStatus::Closed
    );
    let (kept, _) = application::for_request(app.db(), &owner, task.id, Page::default())
        .await
        .unwrap();
    assert_eq!(kept.len(), 2);
}

/// AC-board-4, AC-board-5
#[tokio::test]
async fn other_owner_cannot_manage_request() {
    let app = TestApp::new().await;
    let owner = factories::user(app.db(), "owner@example.com").await;
    let human = factories::user(app.db(), "human@example.com").await;
    let task = factories::request(app.db(), &owner).await;
    let application = apply(&app, &human, &task).await;
    assert!(matches!(
        application::select_for(app.db(), &human, application.id).await,
        Err(AppError::NotFound)
    ));
    assert!(matches!(
        request::close_for(app.db(), &human, task.id).await,
        Err(AppError::NotFound)
    ));
    app.sign_in_as(&human).await;
    app.get(&paths::task_applications(task.id))
        .await
        .assert_status(StatusCode::NOT_FOUND);
}

/// AC-board-5, AC-board-7
#[tokio::test]
async fn application_contacts_are_private() {
    let app = TestApp::new().await;
    let owner = factories::user(app.db(), "owner@example.com").await;
    let human = factories::user(app.db(), "human@example.com").await;
    let task = factories::request(app.db(), &owner).await;
    let submitted = apply(&app, &human, &task).await;
    app.get("/").await.assert_not_contains(&submitted.contact);
    app.get(&paths::task(task.id))
        .await
        .assert_not_contains(&submitted.contact);
    app.sign_in_as(&owner).await;
    let res = app.get(&paths::task_applications(task.id)).await;
    res.assert_ok().assert_text(&submitted.contact);
    assert_eq!(res.headers.get("cache-control").unwrap(), "no-store");
    app.sign_in_as(&human).await;
    app.get(&paths::dashboard())
        .await
        .assert_ok()
        .assert_text(&submitted.contact);
}

/// AC-board-6
#[tokio::test]
async fn agent_api_can_post_and_revocation_ends_access() {
    let app = TestApp::new().await;
    let owner = factories::user(app.db(), "owner@example.com").await;
    let agent = factories::agent(app.db(), &owner).await;
    let (token, raw) = board::models::api_token::issue(
        app.db(),
        &owner,
        "test",
        true,
        Some(chrono::Utc::now() + chrono::Duration::days(1)),
        chrono::Utc::now(),
    )
    .await
    .unwrap();
    let bearer = format!("Bearer {raw}");
    let headers = [("authorization", bearer.as_str())];
    let res=app.send_json("POST", "/api/v1/requests", Some(&serde_json::json!({"agent_id":agent.id,"title":"A human needed","description":"Check the doorway","category":"real-world","location":"Berlin","budget":"40.00"})), &headers).await;
    res.assert_status(StatusCode::CREATED);
    assert_eq!(res.json::<serde_json::Value>()["data"]["status"], "open");
    board::models::api_token::revoke_for(app.db(), &owner, token.id)
        .await
        .unwrap();
    app.send_json("GET", "/api/v1/requests", None, &headers)
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

/// AC-board-6, AC-board-7
#[tokio::test]
async fn agent_hiring_reads_are_private_and_read_only_tokens_cannot_select() {
    let app = TestApp::new().await;
    let owner = factories::user(app.db(), "owner@example.com").await;
    let human = factories::user(app.db(), "human@example.com").await;
    let task = factories::request(app.db(), &owner).await;
    let submitted = apply(&app, &human, &task).await;
    let raw = app.api_token_for(&owner, false).await;
    let bearer = format!("Bearer {raw}");
    let headers = [("authorization", bearer.as_str())];
    let response = app
        .send_json(
            "GET",
            &format!("/api/v1/requests/{}/applications", task.id),
            None,
            &headers,
        )
        .await;
    response.assert_ok().assert_text(&submitted.contact);
    assert_eq!(response.headers.get("cache-control").unwrap(), "no-store");
    app.send_json(
        "POST",
        &format!("/api/v1/applications/{}/selection", submitted.id),
        None,
        &headers,
    )
    .await
    .assert_status(StatusCode::FORBIDDEN);
}

/// AC-board-7
#[tokio::test]
async fn public_request_content_is_escaped() {
    let app = TestApp::new().await;
    let owner = factories::user(app.db(), "owner@example.com").await;
    let mut params = factories::request_params(app.db(), &owner).await;
    params.title = "<script>alert(1)</script>".into();
    let task = factories::request_with(app.db(), &owner, params).await;
    app.get(&paths::task(task.id))
        .await
        .assert_ok()
        .assert_not_contains("<script>alert(1)</script>")
        .assert_text("<script>alert(1)</script>");
}

#[tokio::test]
async fn cancellation_keeps_application_history_and_agent_cannot_be_deleted() {
    let app = TestApp::new().await;
    let owner = factories::user(app.db(), "owner@example.com").await;
    let human = factories::user(app.db(), "human@example.com").await;
    let task = factories::request(app.db(), &owner).await;
    let submitted = apply(&app, &human, &task).await;
    request::destroy_for(app.db(), &owner, task.clone())
        .await
        .unwrap();
    assert_eq!(
        request::find_for(app.db(), &owner, task.id)
            .await
            .unwrap()
            .status,
        request::RequestStatus::Cancelled
    );
    assert!(
        application::find_for(app.db(), &human, submitted.id)
            .await
            .is_ok()
    );
    let agent = board::models::agent::find_for(app.db(), &owner, task.agent_id)
        .await
        .unwrap();
    assert!(
        board::models::agent::destroy_for(app.db(), &owner, agent)
            .await
            .is_err()
    );
}

/// AC-board-6
#[tokio::test]
async fn account_recovery_revokes_agent_api_tokens() {
    let app = TestApp::new().await;
    let owner = factories::user(app.db(), "owner@example.com").await;
    let raw = app.api_token_for(&owner, true).await;
    let txn = board::kernel::db::begin(app.db()).await.unwrap();
    board::controllers::auth::revoke_everything(&txn, owner.id)
        .await
        .unwrap();
    txn.commit().await.unwrap();
    let bearer = format!("Bearer {raw}");
    app.send_json(
        "GET",
        "/api/v1/me",
        None,
        &[("authorization", bearer.as_str())],
    )
    .await
    .assert_status(StatusCode::UNAUTHORIZED);
}
