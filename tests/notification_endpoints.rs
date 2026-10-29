mod common;

use crate::common::test_db;
use featherboard::{app_router, session_store};
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::Session;
use tower_sessions_sqlx_store::SqliteStore;

async fn session_cookie(db: &sqlx::SqlitePool, user_id: i64) -> String {
    session_store(db).await;
    let store = Arc::new(SqliteStore::new(db.clone()));
    let session = Session::new(None, store, None);
    session.insert("user_id", user_id).await.unwrap();
    session.save().await.unwrap();
    session.id().unwrap().to_string()
}

//noinspection SqlResolve
#[tokio::test]
async fn creating_a_comment_persists_it_through_the_router() {
    let db = test_db().await;
    sqlx::query(
        "INSERT INTO users (id, org_id, provider, provider_user_id, email, name, is_admin)
         VALUES (1, 1, 'github', '1', 'user@example.com', 'User', 0)",
    )
    .execute(&db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ideas (id, org_id, title, description, user_id)
         VALUES (1, 1, 'An idea', 'A description', 1)",
    )
    .execute(&db)
    .await
    .unwrap();

    let cookie = session_cookie(&db, 1).await;
    let app = app_router(db.clone(), session_store(&db).await, None, None);
    let request = axum::http::Request::builder()
        .uri("/default/ideas/1/comments")
        .method("POST")
        .header("Cookie", format!("id={cookie}"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(
            "author_name=User&body=Useful+feedback",
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
    let body: (String, String) =
        sqlx::query_as("SELECT author_name, body FROM comments WHERE idea_id = 1")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(body, ("User".to_string(), "Useful feedback".to_string()));
}

//noinspection SqlResolve
#[tokio::test]
async fn admin_status_change_persists_through_the_router() {
    let db = test_db().await;
    sqlx::query(
        "INSERT INTO users (id, org_id, provider, provider_user_id, email, name, is_admin)
         VALUES (1, 1, 'github', '1', 'admin@example.com', 'Admin', 1)",
    )
    .execute(&db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ideas (id, org_id, title, description, user_id)
         VALUES (1, 1, 'An idea', 'A description', NULL)",
    )
    .execute(&db)
    .await
    .unwrap();

    let cookie = session_cookie(&db, 1).await;
    let app = app_router(db.clone(), session_store(&db).await, None, None);
    let request = axum::http::Request::builder()
        .uri("/default/ideas/1/status")
        .method("POST")
        .header("Cookie", format!("id={cookie}"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from("status=planned"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
    let status: String = sqlx::query_scalar("SELECT status FROM ideas WHERE id = 1")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(status, "planned");
}

#[tokio::test]
async fn user_can_update_notification_preferences() {
    let db = test_db().await;
    sqlx::query(
        "INSERT INTO users (id, org_id, provider, provider_user_id, email, name, is_admin)
         VALUES (1, 1, 'github', '1', 'user@example.com', 'User', 0)",
    )
    .execute(&db)
    .await
    .unwrap();

    let cookie = session_cookie(&db, 1).await;
    let app = app_router(db.clone(), session_store(&db).await, None, None);
    let request = axum::http::Request::builder()
        .uri("/default/settings/user/notifications")
        .method("POST")
        .header("Cookie", format!("id={cookie}"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from("notify_comments=on"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
    let preferences: (bool, bool) =
        sqlx::query_as("SELECT notify_comments, notify_status_changes FROM users WHERE id = 1")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(preferences, (true, false));
}
