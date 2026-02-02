//! 使用示例：bamboo-tool 和 bamboo-skill
//!
//! 运行: cargo run --example usage_example

use bamboo_skill::{SkillManager, SkillParser};
use bamboo_tool::{ToolExecutor, ToolRunner, ToolDef, ArgDef, ToolRegistry};
use bamboo_tool::registry::InMemoryToolRegistry;
use bamboo_tool::types::ArgType;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Bamboo Skill & Tool System Demo ===\n");

    // ============ bamboo-tool 示例 ============
    println!("1. Tool Registry Example");
    
    let registry = InMemoryToolRegistry::new();
    
    // 注册一个工具（使用 /bin/echo）
    let tool = ToolDef {
        name: "echo".to_string(),
        description: Some("Echo a message".to_string()),
        command: "/bin/echo".to_string(),
        args: vec![
            ArgDef {
                name: "message".to_string(),
                arg_type: ArgType::String,
                required: true,
                default: None,
                description: Some("Message to echo".to_string()),
            }
        ],
    };
    
    registry.register(tool.clone()).await?;
    println!("   ✓ Registered tool: {}", tool.name);
    println!("   ✓ Total tools: {}", registry.len());

    // 执行工具
    println!("\n2. Tool Execution Example");
    let executor = ToolExecutor::new().with_timeout(10);
    let args = std::collections::HashMap::from([
        ("message".to_string(), json!("Hello from bamboo-tool!")),
    ]);
    
    let result = executor.execute(&tool, args).await?;
    println!("   ✓ Success: {}", result.success);
    println!("   ✓ Output: {}", result.output.trim());
    println!("   ✓ Duration: {}ms", result.duration_ms);

    // ============ bamboo-skill 示例 ============
    println!("\n3. Skill Parser Example");
    
    let skill_md = r#"---
name: web-search
version: 0.1.0
description: Search the web using DuckDuckGo
author: Bamboo Team
tools:
  - name: search
    description: Search for a query
    command: tools/search.sh
    args:
      - name: query
        type: string
        required: true
        description: The search query
      - name: limit
        type: number
        required: false
        default: 10
        description: Max results
---

# Web Search Skill

This skill provides web search capabilities.

## Usage

The assistant can search the web when it needs current information.
"#;

    let parser = SkillParser::new();
    let skill = parser.parse(skill_md, "/tmp/skills/web-search/SKILL.md")?;
    
    println!("   ✓ Parsed skill: {}", skill.name);
    println!("   ✓ Version: {}", skill.manifest.version);
    println!("   ✓ Tools: {}", skill.tools.len());
    println!("   ✓ Has system prompt: {}", skill.system_prompt.is_some());
    
    if let Some(tool) = skill.find_tool("search") {
        println!("   ✓ Found tool: {} with {} args", tool.name, tool.args.len());
    }

    // ============ Skill Manager 示例 ============
    println!("\n4. Skill Manager Example");
    
    // 创建一个临时目录来模拟 skills 目录
    let temp_dir = tempfile::tempdir()?;
    let skills_dir = temp_dir.path().join("skills");
    std::fs::create_dir(&skills_dir)?;
    
    // 创建一个 skill 目录和 SKILL.md
    let skill_dir = skills_dir.join("file-ops");
    std::fs::create_dir(&skill_dir)?;
    
    let file_ops_skill = r#"---
name: file-ops
version: 1.0.0
description: File operations skill
author: Demo
tools:
  - name: read
    description: Read a file
    command: cat
  - name: list
    description: List directory
    command: ls
---

# File Operations

Provides file read and list operations.
"#;
    
    std::fs::write(skill_dir.join("SKILL.md"), file_ops_skill)?;
    
    // 初始化 SkillManager
    let manager = SkillManager::new(&skills_dir);
    manager.initialize().await?;
    
    println!("   ✓ Skills loaded: {}", manager.skill_count());
    
    if let Some(skill) = manager.get_skill("file-ops") {
        println!("   ✓ Skill '{}': {} tools", skill.name, skill.tools.len());
    }
    
    let all_tools = manager.get_all_tools();
    println!("   ✓ Total tools available: {}", all_tools.len());
    
    // 热重载演示
    println!("\n5. Hot Reload Example");
    println!("   (File watcher is running in background)");
    
    // 处理一次文件系统事件（检查是否有变化）
    manager.process_events().await?;
    println!("   ✓ Processed file system events");
    
    // 清理
    manager.shutdown().await?;
    println!("   ✓ Manager shut down gracefully");
    
    println!("\n=== Demo Complete ===");
    Ok(())
}
