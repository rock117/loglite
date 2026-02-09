use chrono::{DateTime, FixedOffset, Utc};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, Set};

use crate::entities::prelude::*;
use crate::models::{AppInfo, CreateAppRequest};
use crate::state::AppState;
use crate::utils::generate_app_id;

fn default_ts() -> DateTimeWithTimeZone {
    let utc: DateTime<Utc> = Utc::now();
    utc.with_timezone(&FixedOffset::east_opt(0).unwrap())
}

/// Create an application.
#[post("/apps", data = "<payload>")]
pub async fn create_app(
    state: &State<AppState>,
    payload: Json<CreateAppRequest>,
) -> Result<Json<AppInfo>, Status> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(Status::BadRequest);
    }

    let app_id = generate_app_id(name);
    let created_at = default_ts();

    let model = crate::entities::apps::ActiveModel {
        app_id: Set(app_id.clone()),
        name: Set(name.to_string()),
        created_at: Set(created_at),
    }
    .insert(state.db.as_ref())
    .await
    .map_err(|_| Status::InternalServerError)?;

    Ok(Json(AppInfo {
        app_id: model.app_id,
        name: model.name,
        created_at: model.created_at,
    }))
}

/// List all applications.
#[get("/apps")]
pub async fn list_apps(state: &State<AppState>) -> Result<Json<Vec<AppInfo>>, Status> {
    let apps = App::find()
        .order_by_desc(crate::entities::apps::Column::CreatedAt)
        .all(state.db.as_ref())
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(
        apps.into_iter()
            .map(|a| AppInfo {
                app_id: a.app_id,
                name: a.name,
                created_at: a.created_at,
            })
            .collect(),
    ))
}
