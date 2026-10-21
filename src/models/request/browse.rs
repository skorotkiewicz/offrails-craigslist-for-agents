//! Public request reads. These return only intentionally public records and agent names.
use sea_orm::{Condition, QueryOrder, QuerySelect, entity::prelude::*};
use serde::Deserialize;

use super::{Column, Entity, Model, RequestStatus};
use crate::{
    error::AppError,
    models::{Agent, Errors, Page},
};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BoardFilter {
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub location: String,
}

impl BoardFilter {
    pub fn validate(&self) -> Result<(), AppError> {
        if [&self.q, &self.category, &self.location]
            .iter()
            .any(|s| s.chars().count() > 200)
        {
            return Err(AppError::Invalid(Errors::single(
                "search",
                "must be at most 200 characters",
            )));
        }
        Ok(())
    }
}

fn pattern(text: &str) -> String {
    format!(
        "%{}%",
        text.trim()
            .to_lowercase()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    )
}

#[tracing::instrument(skip_all)]
pub async fn browse(
    db: &impl ConnectionTrait,
    filter: &BoardFilter,
    page: Page,
) -> Result<(Vec<(Model, Option<Agent>)>, bool), AppError> {
    use sea_orm::sea_query::{ExprTrait, Func, LikeExpr};
    filter.validate()?;
    let mut query = Entity::find().filter(Column::Status.eq(RequestStatus::Open));
    if !filter.q.trim().is_empty() {
        let search = pattern(&filter.q);
        query = query.filter(
            Condition::any()
                .add(
                    Expr::expr(Func::lower(Expr::col((Entity, Column::Title))))
                        .like(LikeExpr::new(search.clone()).escape('\\')),
                )
                .add(
                    Expr::expr(Func::lower(Expr::col((Entity, Column::Description))))
                        .like(LikeExpr::new(search).escape('\\')),
                ),
        );
    }
    if !filter.category.is_empty() {
        query = query.filter(Column::Category.eq(filter.category.trim()));
    }
    if !filter.location.trim().is_empty() {
        query = query.filter(
            Expr::expr(Func::lower(Expr::col((Entity, Column::Location))))
                .like(LikeExpr::new(pattern(&filter.location)).escape('\\')),
        );
    }
    let mut rows = query
        .order_by_desc(Column::CreatedAt)
        .order_by_desc(Column::Id)
        .limit(page.per() + 1)
        .offset(page.offset())
        .find_also_related(super::super::agent::Entity)
        .all(db)
        .await?;
    let more = rows.len() as u64 > page.per();
    rows.truncate(page.per() as usize);
    Ok((rows, more))
}

#[tracing::instrument(skip_all, fields(id))]
pub async fn public_request(
    db: &impl ConnectionTrait,
    id: i64,
) -> Result<(Model, Option<Agent>), AppError> {
    Entity::find_by_id(id)
        .find_also_related(super::super::agent::Entity)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)
}
