use axum::Json;
use serde_json::json;

pub(crate) async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "alpaca-mock",
    }))
}
