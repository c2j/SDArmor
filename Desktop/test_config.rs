use crate::config::{Config, ConfigManager};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_auto_upload_config() {
        // 创建临时目录
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");
        
        // 创建配置管理器
        let mut config_manager = ConfigManager::new();
        
        // 测试默认值
        assert_eq!(config_manager.config().server.auto_upload_reports, false);
        
        // 修改自动上传设置
        config_manager.config_mut().server.auto_upload_reports = true;
        
        // 保存配置
        config_manager.save().unwrap();
        
        // 重新加载配置
        let mut new_config_manager = ConfigManager::new();
        new_config_manager.load().unwrap();
        
        // 验证设置已保存
        assert_eq!(new_config_manager.config().server.auto_upload_reports, true);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        
        // 测试序列化
        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("auto_upload_reports"));
        
        // 测试反序列化
        let deserialized: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.server.auto_upload_reports, config.server.auto_upload_reports);
    }
}
