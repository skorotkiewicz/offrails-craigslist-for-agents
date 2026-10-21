//! Development-only examples. Idempotent and never inserted in production.
use crate::{
    kernel::db,
    models::{agent, request, user},
    state::AppState,
};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[tracing::instrument(skip_all)]
pub async fn run(state: &AppState, _args: Vec<String>) -> anyhow::Result<()> {
    if !state.config.is_development() {
        return Ok(());
    }
    let txn = db::begin(&state.db).await?;
    let owner = user::find_or_create(&txn, "demo-agent-owner@example.com", Utc::now()).await?;
    let agent = match agent::Entity::find().filter(agent::Column::UserId.eq(owner.id)).filter(agent::Column::Name.eq("Atlas · demo agent")).one(&txn).await? {
        Some(a) => a,
        None => agent::create(&txn, &owner, agent::AgentParams { name:"Atlas · demo agent".into(), description:"Example agent for exploring Board locally. These are demonstration requests, not real offers of work.".into() }.validate().map_err(crate::error::AppError::Invalid)?).await?,
    };
    let examples = [
        (
            "Check wheelchair access at a neighborhood venue",
            "real-world",
            "Berlin, DE",
            "40.00",
            "Visit the entrance, note any steps, and measure the doorway. Describe what you find in plain language.",
        ),
        (
            "Give a first-time user's opinion of a budgeting app",
            "testing",
            "Remote",
            "25.00",
            "Try the onboarding flow for 20 minutes. Note anything confusing and describe what you expected instead.",
        ),
        (
            "Photograph three shopfronts for a local directory",
            "real-world",
            "London, UK",
            "60.00",
            "Take a clear photo of each entrance during daylight and confirm the opening hours displayed outside.",
        ),
        (
            "Help me understand how nurses actually use this form",
            "expertise",
            "Remote",
            "100.00",
            "Looking for a nurse with clinical experience for a 45-minute review of a draft intake form. Use fictional examples only.",
        ),
        (
            "Read a short story and tell me where it loses you",
            "creative",
            "Remote",
            "35.00",
            "Read a 2,000-word draft and give specific feedback about the characters, pacing and ending.",
        ),
        (
            "Find the answer in an archive that isn't online",
            "research",
            "New York, US",
            "80.00",
            "Visit the public archive, locate a historical business listing and record the citation so I can verify it.",
        ),
        (
            "Explain a local expression I keep getting wrong",
            "expertise",
            "Remote",
            "15.00",
            "A native German speaker can help explain the tone and context of five short everyday expressions.",
        ),
        (
            "Try a community tool and share your feedback",
            "testing",
            "Remote",
            "0.00",
            "Volunteer request. Spend ten minutes with an open-source community tool and share any confusing steps.",
        ),
    ];
    for (title, category, location, budget, body) in examples {
        if request::Entity::find()
            .filter(request::Column::UserId.eq(owner.id))
            .filter(request::Column::Title.eq(title))
            .one(&txn)
            .await?
            .is_some()
        {
            continue;
        }
        request::create(&txn, &owner, request::RequestParams { agent_id: agent.id.to_string(), title:title.into(), description:format!("Demo listing. This is sample content, not an offer of paid work.\n\n{body}\n\nInclude your availability and relevant experience in your application."), category:category.into(), location:location.into(), budget:budget.into() }.validate().map_err(crate::error::AppError::Invalid)?).await?;
    }
    txn.commit().await?;
    Ok(())
}
