use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use bamboo_prompt::SystemPrompt;

#[derive(Debug, Deserialize)]
pub struct CreatePromptRequest {
    pub name: String,
    pub content: String,
    pub category: Option<String>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePromptRequest {
    pub name: Option<String>,
    pub content: Option<String>,
    pub category: Option<String>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PromptListResponse {
    pub prompts: Vec<SystemPrompt>,
}

#[derive(Debug, Serialize)]
pub struct PromptResponse {
    pub prompt: SystemPrompt,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn list_prompts(state: web::Data<AppState>) -> HttpResponse {
    match state.prompt_manager.list_prompts().await {
        Ok(prompts) => HttpResponse::Ok().json(PromptListResponse { prompts }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: e.to_string(),
        }),
    }
}

pub async fn create_prompt(
    state: web::Data<AppState>,
    body: web::Json<CreatePromptRequest>,
) -> HttpResponse {
    let req = body.into_inner();
    
    if req.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Name cannot be empty".to_string(),
        });
    }
    
    let prompt = SystemPrompt::new(
        "",
        req.name,
        req.content,
        req.category.unwrap_or_else(|| "general".to_string()),
    );
    
    match state.prompt_manager.create_prompt(prompt).await {
        Ok(prompt) => HttpResponse::Ok().json(PromptResponse { prompt }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: e.to_string(),
        }),
    }
}

pub async fn update_prompt(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<UpdatePromptRequest>,
) -> HttpResponse {
    let id = path.into_inner();
    let req = body.into_inner();
    
    let existing = match state.prompt_manager.get_prompt(&id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse {
                error: format!("Prompt not found: {}", id),
            });
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: e.to_string(),
            });
        }
    };
    
    let updated = SystemPrompt {
        id: existing.id,
        name: req.name.unwrap_or(existing.name),
        content: req.content.unwrap_or(existing.content),
        category: req.category.unwrap_or(existing.category),
        is_default: req.is_default.unwrap_or(existing.is_default),
        is_custom: existing.is_custom,
    };
    
    match state.prompt_manager.update_prompt(updated).await {
        Ok(prompt) => HttpResponse::Ok().json(PromptResponse { prompt }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: e.to_string(),
        }),
    }
}

pub async fn delete_prompt(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let id = path.into_inner();
    
    match state.prompt_manager.delete_prompt(&id).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({ "success": true })),
        Err(bamboo_prompt::PromptError::NotFound(_)) => {
            HttpResponse::NotFound().json(ErrorResponse {
                error: format!("Prompt not found: {}", id),
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: e.to_string(),
        }),
    }
}

pub async fn set_default_prompt(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let id = path.into_inner();
    
    match state.prompt_manager.set_default(&id).await {
        Ok(prompt) => HttpResponse::Ok().json(PromptResponse { prompt }),
        Err(bamboo_prompt::PromptError::NotFound(_)) => {
            HttpResponse::NotFound().json(ErrorResponse {
                error: format!("Prompt not found: {}", id),
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: e.to_string(),
        }),
    }
}
