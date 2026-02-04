//! bamboo-masking 使用示例
//!
//! 运行: cargo run --example usage

use bamboo_masking::{MaskingConfig, MaskingConfigManager, MaskingRule, RuleType};

#[tokio::main]
async fn main() {
    // 示例 1: 使用默认配置
    println!("=== 示例 1: 默认配置 ===");
    let config = MaskingConfig::default();
    let text = "password: mySecret123, api_key: sk-abc123xyz";
    println!("原始: {}", text);
    println!("脱敏: {}", config.apply_to_text(text));
    println!();

    // 示例 2: 自定义配置
    println!("=== 示例 2: 自定义配置 ===");
    let mut config = MaskingConfig {
        enabled: true,
        rules: vec![],
    };
    
    // 添加信用卡规则
    config.rules.push(MaskingRule {
        id: "credit_card".to_string(),
        name: "Credit Card".to_string(),
        rule_type: RuleType::Regex,
        pattern: r"\b\d{4}-\d{4}-\d{4}-\d{4}\b".to_string(),
        replacement: "****-****-****-****".to_string(),
        enabled: true,
    });
    
    // 添加关键字规则
    config.rules.push(MaskingRule {
        id: "confidential".to_string(),
        name: "Confidential".to_string(),
        rule_type: RuleType::Keyword,
        pattern: "机密".to_string(),
        replacement: "[REDACTED]".to_string(),
        enabled: true,
    });
    
    let text = "我的信用卡是 1234-5678-9012-3456，包含机密信息";
    println!("原始: {}", text);
    println!("脱敏: {}", config.apply_to_text(text));
    println!();

    // 示例 3: 验证规则
    println!("=== 示例 3: 验证规则 ===");
    match config.validate() {
        Ok(_) => println!("所有规则验证通过!"),
        Err(e) => println!("验证失败: {}", e),
    }
    println!();

    // 示例 4: 配置管理器（异步）
    println!("=== 示例 4: 配置管理器 ===");
    let temp_path = std::env::temp_dir().join("masking_example.json");
    
    // 加载或创建配置
    let manager = MaskingConfigManager::load(&temp_path).await.unwrap();
    println!("配置加载成功，规则数: {}", manager.get().read().await.rules.len());
    
    // 更新配置
    manager.update(|config| {
        config.rules.push(MaskingRule {
            id: "custom".to_string(),
            name: "Custom".to_string(),
            rule_type: RuleType::Regex,
            pattern: r"custom_\d+".to_string(),
            replacement: "[CUSTOM]".to_string(),
            enabled: true,
        });
    }).await.unwrap();
    println!("配置已更新，新规则数: {}", manager.get().read().await.rules.len());
    
    // 清理
    std::fs::remove_file(&temp_path).unwrap();
    
    println!("\n所有示例完成!");
}
