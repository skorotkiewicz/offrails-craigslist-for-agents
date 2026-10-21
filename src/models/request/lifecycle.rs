use super::{Column, Entity, Model, RequestStatus};
use crate::{
    error::AppError,
    kernel::db,
    models::{Errors, User},
};
use chrono::Utc;
use sea_orm::{DatabaseConnection, entity::prelude::*};

/// Terminal transitions preserve the request and all application records.
#[tracing::instrument(skip_all, fields(id))]
pub async fn close_for(db: &DatabaseConnection, user: &User, id: i64) -> Result<Model, AppError> {
    let txn = db::begin(db).await?;
    super::find_for(&txn, user, id).await?;
    Entity::update_many()
        .col_expr(Column::Status, Expr::value(RequestStatus::Closed))
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(Column::Id.eq(id))
        .filter(Column::UserId.eq(user.id))
        .filter(Column::Status.is_in([RequestStatus::Open, RequestStatus::Assigned]))
        .exec(&txn)
        .await?;
    let row = super::find_for(&txn, user, id).await?;
    if row.status == RequestStatus::Cancelled {
        return Err(AppError::Invalid(Errors::single("request", "is cancelled")));
    }
    txn.commit().await?;
    Ok(row)
}
