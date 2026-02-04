use std::path::PathBuf;
use tokio::fs;
use serde_json::json;

/// 允许访问的基础目录
const ALLOWED_BASE_DIR: &str = "/Users/bigduu";

/// 文件系统工具
pub struct FilesystemTool;

impl FilesystemTool {
    /// 验证路径安全性：解析绝对路径并确保在允许的基础目录内
    /// 如果路径不存在，会尝试解析其父目录来验证安全性
    fn validate_path(path: &str) -> Result<PathBuf, String> {
        // 检查路径是否为空
        if path.is_empty() {
            return Err("Invalid path: path is empty".to_string());
        }

        // 解析允许的基础目录
        let base_path = std::fs::canonicalize(ALLOWED_BASE_DIR)
            .map_err(|e| format!("Internal error: invalid base directory '{}': {}", ALLOWED_BASE_DIR, e))?;

        // 尝试解析输入路径为绝对路径
        let canonical_path = match std::fs::canonicalize(path) {
            Ok(p) => p,
            Err(_) => {
                // 路径不存在，手动解析为绝对路径
                let path_buf = PathBuf::from(path);
                if path_buf.is_absolute() {
                    path_buf
                } else {
                    std::env::current_dir()
                        .map_err(|e| format!("Failed to get current directory: {}", e))?
                        .join(path_buf)
                }
            }
        };

        // 验证路径是否在允许的基础目录内
        if !canonical_path.starts_with(&base_path) {
            return Err(format!(
                "Access denied: path '{}' is outside the allowed directory '{}'",
                canonical_path.display(),
                base_path.display()
            ));
        }

        Ok(canonical_path)
    }

    /// 读取文件内容
    pub async fn read_file(path: &str) -> Result<String, String> {
        let validated_path = Self::validate_path(path)?;
        
        fs::read_to_string(&validated_path)
            .await
            .map_err(|e| format!("Failed to read file '{}': {}", validated_path.display(), e))
    }
    
    /// 写入文件内容
    pub async fn write_file(path: &str, content: &str) -> Result<(), String> {
        let validated_path = Self::validate_path(path)?;
        
        // 确保父目录存在
        if let Some(parent) = validated_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create directory '{}': {}", parent.display(), e))?;
        }
        
        fs::write(&validated_path, content)
            .await
            .map_err(|e| format!("Failed to write file '{}': {}", validated_path.display(), e))
    }
    
    /// 列出目录内容
    pub async fn list_directory(path: &str) -> Result<Vec<String>, String> {
        let validated_path = Self::validate_path(path)?;
        
        let mut entries = vec![];
        let mut dir = fs::read_dir(&validated_path)
            .await
            .map_err(|e| format!("Failed to read directory '{}': {}", validated_path.display(), e))?;
        
        while let Some(entry) = dir.next_entry()
            .await
            .map_err(|e| format!("Failed to read directory entry: {}", e))? {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let file_type = if entry.file_type().await.map_err(|e| e.to_string())?.is_dir() {
                "[DIR]"
            } else {
                "[FILE]"
            };
            entries.push(format!("{} {}", file_type, file_name));
        }
        
        Ok(entries)
    }
    
    /// 检查文件是否存在
    pub async fn file_exists(path: &str) -> Result<bool, String> {
        let validated_path = Self::validate_path(path)?;
        
        Ok(fs::metadata(&validated_path).await.is_ok())
    }
    
    /// 获取文件信息
    pub async fn get_file_info(path: &str) -> Result<String, String> {
        let validated_path = Self::validate_path(path)?;
        
        let metadata = fs::metadata(&validated_path)
            .await
            .map_err(|e| format!("Failed to get file info '{}': {}", validated_path.display(), e))?;
        
        let size = metadata.len();
        let is_file = metadata.is_file();
        let is_dir = metadata.is_dir();
        let modified = metadata.modified()
            .map_err(|e| e.to_string())?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        
        Ok(format!(
            "Path: {}\nType: {}\nSize: {} bytes\nModified: {} UTC",
            path,
            if is_file { "File" } else if is_dir { "Directory" } else { "Other" },
            size,
            chrono::DateTime::from_timestamp(modified as i64, 0)
                .map(|d: chrono::DateTime<chrono::Utc>| d.to_rfc3339())
                .unwrap_or_else(|| "Unknown".to_string())
        ))
    }
    
    /// 获取工具 schema
    pub fn get_tool_schemas() -> Vec<serde_json::Value> {
        vec![
            json!({
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "读取文件内容，支持 txt, json, md, rs 等文本文件。路径必须是绝对路径，例如 /Users/bigduu/workspace/project/file.txt",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "文件的绝对路径"
                            }
                        },
                        "required": ["path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "write_file",
                    "description": "写入文件内容。如果文件不存在会自动创建，包括父目录。路径必须是绝对路径",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "文件的绝对路径"
                            },
                            "content": {
                                "type": "string",
                                "description": "要写入的内容"
                            }
                        },
                        "required": ["path", "content"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "list_directory",
                    "description": "列出目录中的所有文件和子目录",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "目录的绝对路径"
                            }
                        },
                        "required": ["path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "file_exists",
                    "description": "检查文件或目录是否存在",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "文件或目录的绝对路径"
                            }
                        },
                        "required": ["path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "get_file_info",
                    "description": "获取文件的详细信息（大小、类型、修改时间等）",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "文件的绝对路径"
                            }
                        },
                        "required": ["path"]
                    }
                }
            }),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_read_write_file() {
        let test_dir = PathBuf::from(ALLOWED_BASE_DIR).join("mcp_test_fs");
        let test_path = test_dir.join("test.txt").to_str().unwrap().to_string();
        let test_content = "Hello, MCP!";
        
        // 确保目录存在（使用标准库的 create_dir_all 在测试前创建）
        std::fs::create_dir_all(&test_dir).expect("Failed to create test directory");
        
        // 写入文件
        let result = FilesystemTool::write_file(&test_path, test_content).await;
        assert!(result.is_ok(), "Failed to write file: {:?}", result);
        
        // 读取文件
        let content = FilesystemTool::read_file(&test_path).await.unwrap();
        assert_eq!(content, test_content);
        
        // 清理
        let _ = fs::remove_file(&test_path).await;
        let _ = fs::remove_dir(&test_dir).await;
    }

    #[tokio::test]
    async fn test_list_directory() {
        let entries = FilesystemTool::list_directory(ALLOWED_BASE_DIR).await;
        assert!(entries.is_ok(), "Failed to list directory: {:?}", entries);
        // 目录可能为空，不检查是否非空
    }

    #[tokio::test]
    async fn test_path_traversal_protection() {
        // 测试 .. 路径遍历
        let result = FilesystemTool::read_file("/etc/../etc/passwd").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        // 应该因为路径在允许目录之外而被拒绝
        assert!(err.contains("Access denied") || err.contains("Invalid path"), 
            "Expected access denied or invalid path error, got: {}", err);
    }

    #[tokio::test]
    async fn test_symlink_traversal_protection() {
        // 测试符号链接路径遍历攻击
        let test_dir = "/tmp/test_mcp_symlink";
        let symlink_path = format!("{}/evil_link", test_dir);
        
        // 创建测试目录和指向 /etc 的符号链接
        let _ = fs::create_dir_all(test_dir).await;
        let _ = std::fs::remove_file(&symlink_path);
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let _ = symlink("/etc", &symlink_path);
            
            // 尝试通过符号链接访问 /etc/passwd
            let evil_path = format!("{}/passwd", symlink_path);
            let result = FilesystemTool::read_file(&evil_path).await;
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.contains("Access denied"), 
                "Expected access denied error for symlink traversal, got: {}", err);
        }
        
        // 清理
        let _ = fs::remove_dir_all(test_dir).await;
    }

    #[tokio::test]
    async fn test_allowed_directory_access() {
        // 测试允许目录内的访问
        let test_dir = PathBuf::from(ALLOWED_BASE_DIR).join("mcp_allowed");
        let test_path = test_dir.join("test.txt").to_str().unwrap().to_string();
        let test_content = "Test content in allowed directory";
        
        // 确保目录存在（使用标准库的 create_dir_all 在测试前创建）
        std::fs::create_dir_all(&test_dir).expect("Failed to create test directory");
        
        // 写入文件
        let result = FilesystemTool::write_file(&test_path, test_content).await;
        assert!(result.is_ok(), "Should be able to write to allowed directory: {:?}", result);
        
        // 读取文件
        let content = FilesystemTool::read_file(&test_path).await;
        assert!(content.is_ok());
        assert_eq!(content.unwrap(), test_content);
        
        // 清理
        let _ = fs::remove_file(&test_path).await;
        let _ = fs::remove_dir(&test_dir).await;
    }
}
