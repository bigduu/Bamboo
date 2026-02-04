//! bamboo-masking 使用示例
//!
//! 运行: cargo run --example usage

use bamboo_masking::{MaskingEngine, MaskingConfig, MaskingRule, Message};

fn main() {
    // 示例 1: 使用默认配置
    println!("=== 示例 1: 默认配置 ===");
    let engine = MaskingEngine::default();
    let text = "password: mySecret123, api_key: sk-abc123xyz";
    println!("原始: {}", text);
    println!("脱敏: {}", engine.apply(text));
    println!();

    // 示例 2: 自定义配置
    println!("=== 示例 2: 自定义配置 ===");
    let mut config = MaskingConfig::empty();
    config.add_rule(
        MaskingRule::new_regex("credit_card", r"\b\d{4}-\d{4}-\d{4}-\d{4}\b")
            .with_replacement("****-****-****-****")
            .with_description("信用卡号脱敏")
    );
    config.add_rule(
        MaskingRule::new_keyword("confidential", "机密")
            .with_replacement("[REDACTED]")
    );
    
    let engine = MaskingEngine::new(config);
    let text = "我的信用卡是 1234-5678-9012-3456，包含机密信息";
    println!("原始: {}", text);
    println!("脱敏: {}", engine.apply(text));
    println!();

    // 示例 3: 消息列表脱敏
    println!("=== 示例 3: 消息列表脱敏 ===");
    let engine = MaskingEngine::default();
    let messages = vec![
        Message::new("user", "我的密码是 secret123"),
        Message::new("assistant", "收到，secret123 已记录"),
    ];
    println!("原始消息:");
    for msg in &messages {
        println!("  [{}]: {}", msg.role, msg.content);
    }
    
    let masked = engine.apply_to_messages(&messages);
    println!("脱敏后:");
    for msg in &masked {
        println!("  [{}]: {}", msg.role, msg.content);
    }
    println!();

    // 示例 4: 检查敏感信息
    println!("=== 示例 4: 检查敏感信息 ===");
    let engine = MaskingEngine::default();
    let texts = vec![
        "这是一段普通文本",
        "包含 password: secret",
        "包含 token: abc123",
    ];
    for text in texts {
        let has_sensitive = engine.contains_sensitive(text);
        println!("'{}' 包含敏感信息: {}", text, has_sensitive);
    }
    println!();

    // 示例 5: 查找敏感信息
    println!("=== 示例 5: 查找敏感信息 ===");
    let engine = MaskingEngine::default();
    let text = "password: pass123 和 email: test@example.com";
    let matches = engine.find_sensitive(text);
    println!("文本: {}", text);
    println!("找到的敏感信息:");
    for m in matches {
        println!("  - 规则: {}, 匹配: '{}', 位置: {}-{}", 
            m.rule_name, m.matched, m.start, m.end);
    }
    println!();

    // 示例 6: 配置持久化
    println!("=== 示例 6: 配置持久化 ===");
    let mut config = MaskingConfig::empty();
    config.add_rule(MaskingRule::new_regex("custom", r"custom_\d+").with_replacement("[CUSTOM]"));
    
    // 保存到临时文件
    let temp_path = std::env::temp_dir().join("masking_example.json");
    config.save_to_file(&temp_path).unwrap();
    println!("配置已保存到: {:?}", temp_path);
    
    // 从文件加载
    let loaded = MaskingConfig::load_from_file(&temp_path).unwrap();
    println!("加载的配置规则数: {}", loaded.rules.len());
    
    // 清理
    std::fs::remove_file(&temp_path).unwrap();
    
    println!("\n所有示例完成!");
}
