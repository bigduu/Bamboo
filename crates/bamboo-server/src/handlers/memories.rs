use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use bamboo_memory::models::{Memory, SessionMemory};

#[derive(Debug, Serialize)]
pub struct MemoryListResponse {
    pub memories: Vec<Memory>,
}

#[derive(Debug, Serialize)]
pub struct SessionMemoryResponse {
    pub session_memory: SessionMemory,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn list_memories(state: web::Data<AppState>) -> HttpResponse {
    match state.memory_manager.list_memories().await {
        Ok(memories) => HttpResponse::Ok().json(MemoryListResponse { memories }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: e.to_string(),
        }),
    }
}

pub async fn get_session_memory(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    
    match state.memory_manager.get_session_memory(&session_id).await {
        Ok(session_memory) => {
            HttpResponse::Ok().json(SessionMemoryResponse { session_memory })
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: e.to_string(),
        }),
    }
}
