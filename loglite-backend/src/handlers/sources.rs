use chrono::{DateTime, FixedOffset, Utc};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::entities::prelude::*;
use crate::models::{CreateSourceRequest, SourceInfo, UpdateSourceRequest};
use crate::state::AppState;

fn default_ts() -> DateTimeWithTimeZone {
    let utc: DateTime<Utc> = Utc::now();
    utc.with_timezone(&FixedOffset::east_opt(0).unwrap())
}

/// Create a log source for an application.
#[post("/sources", data = "<payload>")]
pub async fn create_source(
    state: &State<AppState>,
    payload: Json<CreateSourceRequest>,
) -> Result<Json<SourceInfo>, Status> {
    let created_at = default_ts();

    let model = crate::entities::app_sources::ActiveModel {
        id: Set(0), // Auto-increment
        app_id: Set(payload.app_id.clone()),
        kind: Set(payload.kind.clone()),
        path: Set(payload.path.clone()),
        recursive: Set(payload.recursive.unwrap_or(false)),
        encoding: Set(payload.encoding.clone().unwrap_or_else(|| "utf-8".to_string())),
        include_glob: Set(payload.include_glob.clone()),
        exclude_glob: Set(payload.exclude_glob.clone()),
        enabled: Set(payload.enabled.unwrap_or(true)),
        created_at: Set(created_at),
    }
    .insert(state.db.as_ref())
    .await
    .map_err(|_| Status::InternalServerError)?;

    Ok(Json(SourceInfo {
        id: model.id,
        app_id: model.app_id,
        kind: model.kind,
        path: model.path,
        recursive: model.recursive,
        encoding: model.encoding,
        include_glob: model.include_glob,
        exclude_glob: model.exclude_glob,
        enabled: model.enabled,
        created_at: model.created_at,
    }))
}

/// List all sources for an application.
#[get("/sources?<app_id>")]
pub async fn list_sources(
    state: &State<AppState>,
    app_id: Option<String>,
) -> Result<Json<Vec<SourceInfo>>, Status> {
    let mut query = AppSource::find();

    if let Some(app_id) = app_id {
        query = query.filter(crate::entities::app_sources::Column::AppId.eq(app_id));
    }

    let sources = query
        .order_by_desc(crate::entities::app_sources::Column::CreatedAt)
        .all(state.db.as_ref())
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(
        sources
            .into_iter()
            .map(|s| SourceInfo {
                id: s.id,
                app_id: s.app_id,
                kind: s.kind,
                path: s.path,
                recursive: s.recursive,
                encoding: s.encoding,
                include_glob: s.include_glob,
                exclude_glob: s.exclude_glob,
                enabled: s.enabled,
                created_at: s.created_at,
            })
            .collect(),
    ))
}

/// Get a single source by ID.
#[get("/sources/<id>")]
pub async fn get_source(state: &State<AppState>, id: i64) -> Result<Json<SourceInfo>, Status> {
    let source = AppSource::find_by_id(id)
        .one(state.db.as_ref())
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;

    Ok(Json(SourceInfo {
        id: source.id,
        app_id: source.app_id,
        kind: source.kind,
        path: source.path,
        recursive: source.recursive,
        encoding: source.encoding,
        include_glob: source.include_glob,
        exclude_glob: source.exclude_glob,
        enabled: source.enabled,
        created_at: source.created_at,
    }))
}

/// Update a source.
#[put("/sources/<id>", data = "<payload>")]
pub async fn update_source(
    state: &State<AppState>,
    id: i64,
    payload: Json<UpdateSourceRequest>,
) -> Result<Json<SourceInfo>, Status> {
    let source = AppSource::find_by_id(id)
        .one(state.db.as_ref())
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;

    let mut active: crate::entities::app_sources::ActiveModel = source.into();

    if let Some(path) = &payload.path {
        active.path = Set(path.clone());
    }
    if let Some(recursive) = payload.recursive {
        active.recursive = Set(recursive);
    }
    if let Some(encoding) = &payload.encoding {
        active.encoding = Set(encoding.clone());
    }
    if let Some(include_glob) = &payload.include_glob {
        active.include_glob = Set(Some(include_glob.clone()));
    }
    if let Some(exclude_glob) = &payload.exclude_glob {
        active.exclude_glob = Set(Some(exclude_glob.clone()));
    }
    if let Some(enabled) = payload.enabled {
        active.enabled = Set(enabled);
    }

    let updated = active
        .update(state.db.as_ref())
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(SourceInfo {
        id: updated.id,
        app_id: updated.app_id,
        kind: updated.kind,
        path: updated.path,
        recursive: updated.recursive,
        encoding: updated.encoding,
        include_glob: updated.include_glob,
        exclude_glob: updated.exclude_glob,
        enabled: updated.enabled,
        created_at: updated.created_at,
    }))
}

/// Delete a source.
#[delete("/sources/<id>")]
pub async fn delete_source(state: &State<AppState>, id: i64) -> Result<Status, Status> {
    let source = AppSource::find_by_id(id)
        .one(state.db.as_ref())
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;

    let active: crate::entities::app_sources::ActiveModel = source.into();
    active
        .delete(state.db.as_ref())
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Status::NoContent)
}
