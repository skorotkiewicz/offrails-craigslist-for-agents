//! Tenant isolation suite (.offrails/guides/test-standards.md §4), generated from `ROUTES`.
//! Written by `cargo offrails generate`; each scoped resource adds itself at the `samples:`
//! anchors. Keep its shape when you edit it: a new route with an id fails until `fill` knows it.

use axum::http::StatusCode;
use board::models::User;
use board::routes::{Access, ROUTES};

use crate::factories;
use crate::support::TestApp;

// (1) One record of every tenant-owned model.
struct Tenant {
    user: User,
    // agents
    agent: board::models::Agent,
    // requests
    request: board::models::Request,
    // samples: tenant fields
}

async fn tenant(app: &TestApp, name: &str) -> Tenant {
    let db = app.db();
    let user = factories::user(db, &format!("{name}@example.com")).await;
    let agent = factories::agent(db, &user).await; // agents
    let request = factories::request(db, &user).await; // requests
    // samples: tenant records
    Tenant {
        user,
        // agents
        agent,
        // requests
        request,
        // samples: tenant init
    }
}

// (2) (method, route) → status when A uses B's ids, and the AC that says so. Everything else: 404.
// Strict isolation unless a spec shares something.
const EXPECTED: &[(&str, &str, StatusCode, &str)] = &[
    // samples: tenant expected
];

// (3) Ids that aren't a tenant's record, and where they're tested instead.
const NOT_TENANT_IDS: &[(&str, &str)] = &[
    (
        "/account/sessions/{id}/delete",
        "not a record of a tenant-owned model (account::revoke): its part's suite tests it",
    ),
    // requests
    (
        "/api/v1/requests/{id}",
        "a bearer-token API route: api_requests_test::another_users_request_is_404",
    ),
    // agents
    (
        "/api/v1/agents/{id}",
        "a bearer-token API route: api_agents_test::another_users_agent_is_404",
    ),
    // samples: not tenant ids
];

// (4) Whose id each placeholder is. Panics on one it doesn't know.
fn fill(template: &str, t: &Tenant) -> String {
    let mut path = template.to_owned();
    // agents
    if template.starts_with("/agents/") {
        path = path.replace("{id}", &t.agent.id.to_string());
    }
    // requests
    if template.starts_with("/requests/") {
        path = path.replace("{id}", &t.request.id.to_string());
    }
    // samples: tenant fill
    assert!(
        !path.contains('{'),
        "{template} takes an id this suite doesn't know. Teach `fill` in \
         tests/app/tenancy_test.rs whose record it is, or list it in NOT_TENANT_IDS with a reason."
    );
    path
}

fn in_scope(template: &str, access: Access) -> bool {
    access == Access::User
        && template.contains('{')
        && !NOT_TENANT_IDS.iter().any(|(t, _)| *t == template)
}

// (5) A request that would succeed if it were allowed: the lookup is scoped before the body is
// read, so an empty form is enough.
async fn call(app: &TestApp, method: &str, template: &str, path: &str) -> StatusCode {
    match method {
        "GET" => app.get(path).await.status,
        "POST" => app.post_form(path, &[]).await.status,
        m => panic!("{m} {template}: teach `call` this method"),
    }
}

#[tokio::test]
async fn another_tenants_ids_answer_404_or_their_specd_status_on_every_route() {
    let app = TestApp::new().await;
    let a = tenant(&app, "alice").await;
    let b = tenant(&app, "bob").await;
    app.sign_in_as(&a.user).await;
    let mut checked = 0;
    for (method, template, handler, access) in ROUTES {
        if !in_scope(template, *access) {
            continue;
        }
        let (want, why) = EXPECTED
            .iter()
            .find(|(m, t, _, _)| m == method && t == template)
            .map(|(_, _, s, why)| (*s, *why))
            .unwrap_or((StatusCode::NOT_FOUND, "B's records don't exist for A"));
        let got = call(&app, method, template, &fill(template, &b)).await;
        assert_eq!(
            got, want,
            "{method} {template} ({handler}) with another tenant's ids answered \
            {got}, want {want} ({why}). Look records up with `<model>::find_for(db, &user, id)`."
        );
        checked += 1;
    }
    assert!(checked > 0, "no id route was checked; is ROUTES complete?");
    // Nothing of B's changed.
    // agents
    assert_eq!(
        factories::reload_agent(app.db(), b.agent.id).await.as_ref(),
        Some(&b.agent)
    );
    // requests
    assert_eq!(
        factories::reload_request(app.db(), b.request.id)
            .await
            .as_ref(),
        Some(&b.request)
    );
    // samples: tenant unchanged
}

#[tokio::test]
async fn every_expected_entry_is_a_real_route() {
    for (method, template, _, why) in EXPECTED {
        assert!(
            ROUTES
                .iter()
                .any(|(m, t, _, a)| m == method && t == template && in_scope(t, *a)),
            "EXPECTED lists {method} {template} ({why}), which is not an in-scope route"
        );
    }
    for (template, why) in NOT_TENANT_IDS {
        assert!(
            ROUTES.iter().any(|(_, t, _, _)| t == template),
            "NOT_TENANT_IDS lists {template} ({why}), which is not a route"
        );
    }
}

#[tokio::test]
async fn own_ids_work_so_the_404s_are_not_a_blanket() {
    let app = TestApp::new().await;
    let a = tenant(&app, "alice").await;
    app.sign_in_as(&a.user).await;
    for (method, template, _, access) in ROUTES {
        if *method == "GET" && in_scope(template, *access) {
            let got = call(&app, method, template, &fill(template, &a)).await;
            assert!(
                got.is_success() || got.is_redirection(),
                "GET {template} with A's own ids answered {got}"
            );
        }
    }
}
