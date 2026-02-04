use actix_web::{web, HttpResponse};
use bamboo_masking::{MaskingConfig, MaskingError, MaskingRule, RuleType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

const DEFAULT_REPLACEMENT: &str = "[MASKED]";

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

fn error_response(status: actix_web::http::StatusCode, message: impl Into<String>) -> HttpResponse {
    HttpResponse::build(status).json(ErrorResponse {
        error: message.into(),
    })
}

fn masking_error_response(error: MaskingError) -> HttpResponse {
    match error {
        MaskingError::InvalidRegex { .. } => {
            error_response(actix_web::http::StatusCode::BAD_REQUEST, error.to_string())
        }
        _ => error_response(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

/// 获取 masking 配置
pub async fn get_config(state: web::Data<AppState>) -> HttpResponse {
    let config = state.masking.get().read().await.clone();
    HttpResponse::Ok().json(config)
}

/// 更新 masking 配置（全量覆盖）
pub async fn update_config(
    state: web::Data<AppState>,
    request: web::Json<MaskingConfig>,
) -> HttpResponse {
    let new_config = request.into_inner();
    if let Err(err) = new_config.validate() {
        return masking_error_response(err);
    }

    if let Err(err) = state
        .masking
        .update(|config| {
            *config = new_config.clone();
        })
        .await
    {
        return masking_error_response(err);
    }

    HttpResponse::Ok().json(new_config)
}

/// 获取所有规则
pub async fn list_rules(state: web::Data<AppState>) -> HttpResponse {
    let config = state.masking.get().read().await.clone();
    HttpResponse::Ok().json(config.rules)
}

#[derive(Debug, Deserialize)]
pub struct MaskingRuleInput {
    pub id: Option<String>,
    pub name: String,
    pub rule_type: RuleType,
    pub pattern: String,
    pub replacement: Option<String>,
    pub enabled: Option<bool>,
}

impl MaskingRuleInput {
    fn into_rule(self, id: String) -> MaskingRule {
        MaskingRule {
            id,
            name: self.name,
            rule_type: self.rule_type,
            pattern: self.pattern,
            replacement: self
                .replacement
                .unwrap_or_else(|| DEFAULT_REPLACEMENT.to_string()),
            enabled: self.enabled.unwrap_or(true),
        }
    }
}

/// 创建规则
pub async fn create_rule(
    state: web::Data<AppState>,
    request: web::Json<MaskingRuleInput>,
) -> HttpResponse {
    let input = request.into_inner();
    let rule_id = input.id.clone().filter(|id| !id.is_empty()).unwrap_or_else(|| {
        Uuid::new_v4().to_string()
    });
    let rule = input.into_rule(rule_id.clone());

    if let Err(err) = rule.validate() {
        return masking_error_response(err);
    }

    let mut exists = false;
    if let Err(err) = state
        .masking
        .update(|config| {
            exists = config.rules.iter().any(|r| r.id == rule_id);
            if !exists {
                config.rules.push(rule.clone());
            }
        })
        .await
    {
        return masking_error_response(err);
    }

    if exists {
        return error_response(
            actix_web::http::StatusCode::CONFLICT,
            format!("Rule id already exists: {}", rule_id),
        );
    }

    HttpResponse::Created().json(rule)
}

/// 更新规则
pub async fn update_rule(
    state: web::Data<AppState>,
    path: web::Path<String>,
    request: web::Json<MaskingRuleInput>,
) -> HttpResponse {
    let rule_id = path.into_inner();
    let input = request.into_inner();
    if let Some(id) = &input.id {
        if id != &rule_id {
            return error_response(
                actix_web::http::StatusCode::BAD_REQUEST,
                "Rule id in path and body do not match",
            );
        }
    }

    let updated_rule = input.into_rule(rule_id.clone());
    if let Err(err) = updated_rule.validate() {
        return masking_error_response(err);
    }

    let mut found = false;
    if let Err(err) = state
        .masking
        .update(|config| {
            if let Some(rule) = config.rules.iter_mut().find(|r| r.id == rule_id) {
                *rule = updated_rule.clone();
                found = true;
            }
        })
        .await
    {
        return masking_error_response(err);
    }

    if !found {
        return error_response(
            actix_web::http::StatusCode::NOT_FOUND,
            format!("Rule not found: {}", rule_id),
        );
    }

    HttpResponse::Ok().json(updated_rule)
}

/// 删除规则
pub async fn delete_rule(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let rule_id = path.into_inner();
    let mut removed = false;

    if let Err(err) = state
        .masking
        .update(|config| {
            let before = config.rules.len();
            config.rules.retain(|r| r.id != rule_id);
            removed = config.rules.len() != before;
        })
        .await
    {
        return masking_error_response(err);
    }

    if !removed {
        return error_response(
            actix_web::http::StatusCode::NOT_FOUND,
            format!("Rule not found: {}", rule_id),
        );
    }

    HttpResponse::NoContent().finish()
}

#[derive(Debug, Deserialize)]
pub struct TestMaskingRequest {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct TestMaskingResponse {
    pub original: String,
    pub masked: String,
    pub changed: bool,
}

/// 测试 masking 效果
pub async fn test_masking(
    state: web::Data<AppState>,
    request: web::Json<TestMaskingRequest>,
) -> HttpResponse {
    let input = request.into_inner();
    let config = state.masking.get().read().await.clone();
    let masked = config.apply_to_text(&input.text);
    let changed = masked != input.text;

    HttpResponse::Ok().json(TestMaskingResponse {
        original: input.text,
        masked,
        changed,
    })
}
