use rocket::serde::json::Json;

use crate::models::HealthResponse;

/// Health check endpoint.
#[get("/health")]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
