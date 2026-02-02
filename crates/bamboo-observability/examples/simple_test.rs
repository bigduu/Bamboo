//! 简单测试确保代码可以编译

use bamboo_observability::prelude::*;

#[tokio::main]
async fn main() -> bamboo_observability::Result<()> {
    // 测试配置创建
    let config = Config::default()
        .with_log_level("debug")
        .with_json_format(false)
        .with_module_level("bamboo_server", "debug");
    
    println!("Config created: {:?}", config);
    
    // 测试初始化
    let obs = Observability::init(config).await?;
    
    // 测试日志
    info!(target: "bamboo_server", "Test message");
    debug!(target: "bamboo_server", "Debug message");
    
    // 测试指标
    counter!("bamboo_test_total", 1);
    
    // 测试健康检查
    obs.register_health_check("test", bamboo_observability::simple_check("test", true)).await;
    
    // 启动健康服务器
    obs.start_health_server().await?;
    
    println!("Health server started on http://localhost:8080");
    println!("Check /health, /ready, /metrics");
    
    // 等待一会儿
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    
    // 关闭
    obs.shutdown().await?;
    
    println!("Test completed successfully!");
    
    Ok(())
}
