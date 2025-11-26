use std::collections::HashMap;

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Chinese,
}

impl Default for Language {
    fn default() -> Self {
        Self::Chinese // 默认使用中文
    }
}

/// Localization manager
pub struct Localization {
    current_language: Language,
    translations: HashMap<String, HashMap<Language, String>>,
}

impl Default for Localization {
    fn default() -> Self {
        let mut loc = Self {
            current_language: Language::default(),
            translations: HashMap::new(),
        };

        // 初始化翻译
        loc.init_translations();

        loc
    }
}

impl Localization {
    /// Get a localized string by key
    pub fn get(&self, key: &str) -> String {
        if let Some(translations) = self.translations.get(key) {
            if let Some(text) = translations.get(&self.current_language) {
                return text.clone();
            }
        }

        // 如果没有找到翻译，返回键名
        key.to_string()
    }

    /// Set the current language
    pub fn set_language(&mut self, language: Language) {
        self.current_language = language;
    }

    /// Get the current language
    pub fn current_language(&self) -> Language {
        self.current_language
    }

    /// Initialize translations
    fn init_translations(&mut self) {
        // 应用名称
        self.add_translation("app_name", Language::English, "SDChat Security Scanner");
        self.add_translation("app_name", Language::Chinese, "SDChat 安全扫描器");

        // 主要标签
        self.add_translation("tab_scan_config", Language::English, "Scan Configuration");
        self.add_translation("tab_scan_config", Language::Chinese, "扫描配置");

        self.add_translation("tab_dashboard", Language::English, "Dashboard");
        self.add_translation("tab_dashboard", Language::Chinese, "仪表盘");

        self.add_translation("tab_vuln_details", Language::English, "Vulnerability Details");
        self.add_translation("tab_vuln_details", Language::Chinese, "漏洞详情");

        self.add_translation("tab_settings", Language::English, "Settings");
        self.add_translation("tab_settings", Language::Chinese, "设置");

        // 扫描配置页面
        self.add_translation("target_path", Language::English, "Target Path:");
        self.add_translation("target_path", Language::Chinese, "目标路径:");

        self.add_translation("select_directory", Language::English, "Select Directory");
        self.add_translation("select_directory", Language::Chinese, "选择目录");

        self.add_translation("selected", Language::English, "Selected:");
        self.add_translation("selected", Language::Chinese, "已选择:");

        self.add_translation("scanning_rules", Language::English, "Scanning Rules");
        self.add_translation("scanning_rules", Language::Chinese, "扫描规则");

        self.add_translation("update_rules", Language::English, "Update Rules");
        self.add_translation("update_rules", Language::Chinese, "更新规则");

        self.add_translation("download_rules", Language::English, "Download Rules");
        self.add_translation("download_rules", Language::Chinese, "下载规则");

        self.add_translation("no_rules_loaded", Language::English, "No rules loaded. Please download rules first.");
        self.add_translation("no_rules_loaded", Language::Chinese, "未加载规则。请先下载规则。");

        self.add_translation("start_scan", Language::English, "Start Scan");
        self.add_translation("start_scan", Language::Chinese, "开始扫描");

        self.add_translation("please_select_directory", Language::English, "Please select a directory to scan");
        self.add_translation("please_select_directory", Language::Chinese, "请选择要扫描的目录");

        self.add_translation("please_load_rules", Language::English, "Please load scanning rules first");
        self.add_translation("please_load_rules", Language::Chinese, "请先加载扫描规则");

        // 设置页面
        self.add_translation("dark_mode", Language::English, "Dark Mode");
        self.add_translation("dark_mode", Language::Chinese, "深色模式");

        self.add_translation("server_url", Language::English, "Server URL:");
        self.add_translation("server_url", Language::Chinese, "服务器地址:");

        self.add_translation("base_url", Language::English, "Base URL:");
        self.add_translation("base_url", Language::Chinese, "基础URL:");

        self.add_translation("username", Language::English, "Username:");
        self.add_translation("username", Language::Chinese, "用户名:");

        self.add_translation("password", Language::English, "Password:");
        self.add_translation("password", Language::Chinese, "密码:");

        self.add_translation("save_settings", Language::English, "Save Settings");
        self.add_translation("save_settings", Language::Chinese, "保存设置");

        // 状态栏
        self.add_translation("scanning", Language::English, "Scanning...");
        self.add_translation("scanning", Language::Chinese, "扫描中...");

        self.add_translation("files_scanned", Language::English, "Files scanned:");
        self.add_translation("files_scanned", Language::Chinese, "已扫描文件:");

        self.add_translation("rules_matched", Language::English, "Rules matched:");
        self.add_translation("rules_matched", Language::Chinese, "匹配规则:");

        self.add_translation("time_elapsed", Language::English, "Time elapsed:");
        self.add_translation("time_elapsed", Language::Chinese, "已用时间:");

        // 报告相关
        self.add_translation("upload_report", Language::English, "Upload Report");
        self.add_translation("upload_report", Language::Chinese, "上传报告");

        self.add_translation("report_uploaded", Language::English, "Report uploaded successfully");
        self.add_translation("report_uploaded", Language::Chinese, "报告上传成功");

        self.add_translation("upload_failed", Language::English, "Upload failed");
        self.add_translation("upload_failed", Language::Chinese, "上传失败");

        self.add_translation("scan_history", Language::English, "Scan History");
        self.add_translation("scan_history", Language::Chinese, "扫描历史");
    }

    /// Add a translation for a key
    fn add_translation(&mut self, key: &str, language: Language, text: &str) {
        let translations = self.translations
            .entry(key.to_string())
            .or_insert_with(HashMap::new);

        translations.insert(language, text.to_string());
    }
}
