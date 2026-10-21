//! Applications are private to their author and the owner of the request.
use super::{ActiveModel, Column, Entity, Model, NewApplication};
use crate::{
    error::AppError,
    kernel::db,
    models::{Errors, Page, User, request},
};
use chrono::Utc;
use sea_orm::{ActiveValue::Set, DatabaseConnection, QueryOrder, QuerySelect, entity::prelude::*};

#[tracing::instrument(skip_all, fields(id))]
pub async fn find_for(db: &impl ConnectionTrait, user: &User, id: i64) -> Result<Model, AppError> {
    Entity::find_by_id(id)
        .filter(Column::UserId.eq(user.id))
        .one(db)
        .await?
        .ok_or(AppError::NotFound)
}

#[tracing::instrument(skip_all)]
pub async fn all_for(
    db: &impl ConnectionTrait,
    user: &User,
    page: Page,
) -> Result<(Vec<Model>, bool), AppError> {
    let mut rows = Entity::find()
        .filter(Column::UserId.eq(user.id))
        .order_by_desc(Column::CreatedAt)
        .order_by_desc(Column::Id)
        .limit(page.per() + 1)
        .offset(page.offset())
        .all(db)
        .await?;
    let more = rows.len() as u64 > page.per();
    rows.truncate(page.per() as usize);
    Ok((rows, more))
}

#[tracing::instrument(skip_all)]
pub async fn submitted_for(
    db: &impl ConnectionTrait,
    user: &User,
    page: Page,
) -> Result<(Vec<(Model, Option<request::Model>)>, bool), AppError> {
    let mut rows = Entity::find()
        .filter(Column::UserId.eq(user.id))
        .order_by_desc(Column::CreatedAt)
        .order_by_desc(Column::Id)
        .limit(page.per() + 1)
        .offset(page.offset())
        .find_also_related(request::Entity)
        .all(db)
        .await?;
    let more = rows.len() as u64 > page.per();
    rows.truncate(page.per() as usize);
    Ok((rows, more))
}

#[tracing::instrument(skip_all, fields(request_id))]
pub async fn for_request(
    db: &impl ConnectionTrait,
    user: &User,
    request_id: i64,
    page: Page,
) -> Result<(Vec<Model>, bool), AppError> {
    request::find_for(db, user, request_id).await?;
    let mut rows = Entity::find()
        .filter(Column::RequestId.eq(request_id))
        .order_by_asc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .limit(page.per() + 1)
        .offset(page.offset())
        .all(db)
        .await?;
    let more = rows.len() as u64 > page.per();
    rows.truncate(page.per() as usize);
    Ok((rows, more))
}

#[tracing::instrument(skip_all, fields(request_id))]
pub async fn existing_for(
    db: &impl ConnectionTrait,
    user: &User,
    request_id: i64,
) -> Result<Option<Model>, AppError> {
    Ok(Entity::find()
        .filter(Column::RequestId.eq(request_id))
        .filter(Column::UserId.eq(user.id))
        .one(db)
        .await?)
}

/// Lock the request before checking its state. The same row is locked by selection and closure.
#[tracing::instrument(skip_all)]
pub async fn create(
    db: &DatabaseConnection,
    user: &User,
    new: NewApplication,
) -> Result<Model, AppError> {
    let txn = db::begin(db).await?;
    let locked = request::Entity::update_many()
        .col_expr(request::Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(request::Column::Id.eq(new.request_id))
        .filter(request::Column::Status.eq(request::RequestStatus::Open))
        .exec(&txn)
        .await?;
    if locked.rows_affected == 0 {
        return Err(AppError::Invalid(Errors::single(
            "request",
            "is no longer accepting applications",
        )));
    }
    let (task, _) = request::public_request(&txn, new.request_id).await?;
    if task.user_id == user.id {
        return Err(AppError::Invalid(Errors::single(
            "request",
            "is your own request",
        )));
    }
    // ON CONFLICT preserves the Postgres transaction on a duplicate submission.
    let insert = Entity::insert(ActiveModel {
        user_id: Set(user.id),
        request_id: Set(new.request_id),
        pitch: Set(new.pitch),
        contact: Set(new.contact),
        selected_at: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    })
    .on_conflict_do_nothing_on([Column::RequestId, Column::UserId])
    .exec_without_returning(&txn)
    .await?;
    if matches!(insert, sea_orm::TryInsertResult::Conflicted) {
        return Err(AppError::Invalid(Errors::single(
            "application",
            "has already been submitted",
        )));
    }
    let row = existing_for(&txn, user, new.request_id)
        .await?
        .ok_or(AppError::NotFound)?;
    txn.commit().await?;
    Ok(row)
}

/// The request state compare-and-set prevents two owners' concurrent selections from succeeding.
#[tracing::instrument(skip_all, fields(id))]
pub async fn select_for(db: &DatabaseConnection, user: &User, id: i64) -> Result<Model, AppError> {
    let txn = db::begin(db).await?;
    let (application, _) = Entity::find_by_id(id)
        .find_also_related(request::Entity)
        .filter(request::Column::UserId.eq(user.id))
        .one(&txn)
        .await?
        .ok_or(AppError::NotFound)?;
    let result = request::Entity::update_many()
        .col_expr(
            request::Column::Status,
            Expr::value(request::RequestStatus::Assigned),
        )
        .col_expr(request::Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(request::Column::Id.eq(application.request_id))
        .filter(request::Column::UserId.eq(user.id))
        .filter(request::Column::Status.eq(request::RequestStatus::Open))
        .exec(&txn)
        .await?;
    if result.rows_affected != 1 {
        return Err(AppError::Invalid(Errors::single(
            "request",
            "already has a selection or is closed",
        )));
    }
    let mut am: ActiveModel = application.into();
    am.selected_at = Set(Some(Utc::now()));
    am.updated_at = Set(Utc::now());
    let selected = am.update(&txn).await?;
    txn.commit().await?;
    Ok(selected)
}
