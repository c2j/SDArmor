use anyhow::Result;
use eframe::{App, CreationContext, Frame, NativeOptions};
use egui::{Context, CentralPanel, SidePanel, TopBottomPanel, Visuals};
use log::{info, warn, error};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use auth_client::AuthClient;

mod scanner;
mod ui;
mod rules;
mod report;
mod network;
mod config;
mod utils;
mod visualization;
mod localization;
mod auth_client;
mod rules_client;
mod reports_client;



const APP_NAME_EN: &str = "SDChat Security Scanner";
const APP_NAME_CN: &str = "SDChat 安全扫描器";

struct ScannerApp {
    scan_path: Option<PathBuf>,
    selected_rules: Vec<String>,
    scan_in_progress: bool,
    scan_progress: f32,
    scan_results: Option<scanner::ScanResult>,
    ui_state: ui::UiState,
    runtime: tokio::runtime::Runtime,
    scanner: Option<Arc<scanner::Scanner>>,
    progress_rx: Option<tokio::sync::mpsc::Receiver<f32>>,
    rule_manager: Option<rules::RuleManager>,
    rules_loaded: bool,
    downloading_rules: bool,
    download_error: Option<String>,
    heatmap: Option<visualization::Heatmap3D>,
    selected_hotspot: Option<(PathBuf, String)>,
    localization: localization::Localization,
    config_manager: config::ConfigManager,
    scan_start_time: Option<std::time::Instant>,
    files_scanned: usize,
    rules_matched: usize,
    uploading_report: bool,
    upload_status: Option<String>,
    last_rule_check: Option<std::time::Instant>,
    auth_client: Option<Arc<AuthClient>>,
    rules_client: Option<Arc<rules_client::RulesClient>>,
    reports_client: Option<Arc<reports_client::ReportsClient>>,
    show_login_dialog: bool,
    login_username: String,
    login_password: String,
    login_error: Option<String>,
    register_username: String,
    register_email: String,
    register_password: String,
    register_error: Option<String>,
    show_register_dialog: bool,
    show_settings_dialog: bool,
}

impl Default for ScannerApp {
    fn default() -> Self {
        // 创建Tokio运行时
        let runtime = match Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                error!("Failed to initialize Tokio runtime: {}", e);
                panic!("Cannot initialize application without Tokio runtime");
            }
        };

        // 创建配置管理器
        let mut config_manager = config::ConfigManager::new();
        if let Err(e) = config_manager.load() {
            warn!("Failed to load configuration: {}", e);
        }

        // 从配置中加载默认扫描目录
        let default_scan_path = config_manager.config().paths.default_scan_dir.clone();

        // Initialize auth client
        let config_manager_arc = Arc::new(config_manager);
        let auth_client = match AuthClient::new(config_manager_arc.clone()) {
            Ok(client) => Some(Arc::new(client)),
            Err(e) => {
                warn!("Failed to initialize auth client: {}", e);
                None
            }
        };

        // Initialize rules client if auth is available
        let rules_client = auth_client.as_ref().and_then(|auth| {
            match rules_client::RulesClient::new(Arc::clone(auth)) {
                Ok(client) => Some(Arc::new(client)),
                Err(e) => {
                    warn!("Failed to initialize rules client: {}", e);
                    None
                }
            }
        });

        // Initialize reports client if auth is available
        let reports_client = auth_client.as_ref().and_then(|auth| {
            match reports_client::ReportsClient::new(Arc::clone(auth)) {
                Ok(client) => Some(Arc::new(client)),
                Err(e) => {
                    warn!("Failed to initialize reports client: {}", e);
                    None
                }
            }
        });

        Self {
            scan_path: default_scan_path,
            selected_rules: Vec::new(),
            scan_in_progress: false,
            scan_progress: 0.0,
            scan_results: None,
            ui_state: ui::UiState::default(),
            runtime,
            scanner: None,
            progress_rx: None,
            rule_manager: None,
            rules_loaded: false,
            downloading_rules: false,
            download_error: None,
            heatmap: None,
            selected_hotspot: None,
            localization: localization::Localization::default(),
            config_manager: config_manager_arc,
            scan_start_time: None,
            files_scanned: 0,
            rules_matched: 0,
            uploading_report: false,
            upload_status: None,
            last_rule_check: None,
            auth_client,
            show_login_dialog: false,
            login_username: String::new(),
            login_password: String::new(),
            login_error: None,
            register_username: String::new(),
            register_email: String::new(),
            register_password: String::new(),
            register_error: None,
            show_register_dialog: false,
            show_settings_dialog: false,
        }
    }
}

impl App for ScannerApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // 定期检查规则更新（每小时检查一次）
        self.periodic_rule_check();

        // 渲染上传对话框（如果显示）
        if self.ui_state.show_upload_dialog {
            self.render_upload_dialog(ctx);
        }

        // 检查扫描进度更新
        if self.scan_in_progress {
            if let Some(progress_rx) = &mut self.progress_rx {
                // 使用try_recv以避免阻塞UI线程
                match progress_rx.try_recv() {
                    Ok(progress) => {
                        // 更新进度值
                        self.scan_progress = progress;

                        // 检查扫描是否完成
                        if progress >= 1.0 {
                            info!("Scan completed, retrieving results");
                            // 获取扫描结果
                            if let Some(scanner) = &self.scanner {
                                self.scan_results = Some(scanner.get_results());
                                info!("Found {} vulnerabilities", self.scan_results.as_ref().map_or(0, |r| r.hotspots.len()));
                                // 切换到结果面板
                                self.ui_state.active_tab = ui::ActiveTab::Dashboard;

                                // 处理扫描完成后的操作（生成报告、上传等）
                                self.handle_scan_completed();
                            }
                            self.scan_in_progress = false;
                        } else if progress < 0.0 {
                            // 扫描出错
                            error!("Scan encountered an error");
                            self.scan_in_progress = false;
                        } else {
                            // 更新扫描统计信息
                            if let Some(results) = &self.scan_results {
                                self.files_scanned = results.stats.scanned_files;
                                self.rules_matched = results.stats.critical + results.stats.high +
                                                    results.stats.medium + results.stats.low;
                            }
                        }

                        // 请求重绘UI以更新进度
                        ctx.request_repaint();
                    },
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        // 没有新消息，继续等待，适当降低请求重绘频率
                        ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    },
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        // 通道已关闭，扫描可能已完成或出错
                        warn!("Progress channel disconnected");
                        // 尝试获取最终结果
                        if let Some(scanner) = &self.scanner {
                            self.scan_results = Some(scanner.get_results());
                        }
                        self.scan_in_progress = false;
                    }
                }
            }
        }

        // 侧边导航栏
        SidePanel::left("navigation_panel").show(ctx, |ui| {
            ui.heading("导航");
            if ui.button(self.localization.get("tab_scan_config")).clicked() {
                self.ui_state.active_tab = ui::ActiveTab::ScanConfig;
            }
            if ui.button(self.localization.get("tab_dashboard")).clicked() {
                self.ui_state.active_tab = ui::ActiveTab::Dashboard;
            }
            if ui.button(self.localization.get("tab_vuln_details")).clicked() {
                self.ui_state.active_tab = ui::ActiveTab::VulnerabilityDetails;
            }
            if ui.button(self.localization.get("tab_settings")).clicked() {
                self.ui_state.active_tab = ui::ActiveTab::Settings;
            }
        });

        // 主内容区域
        CentralPanel::default().show(ctx, |ui| {
            match self.ui_state.active_tab {
                ui::ActiveTab::ScanConfig => self.ui_scan_config(ui),
                ui::ActiveTab::Dashboard => self.ui_dashboard(ui),
                ui::ActiveTab::VulnerabilityDetails => self.ui_vuln_details(ui),
                ui::ActiveTab::Settings => self.ui_settings(ui),
            }
        });

        // 认证对话框
        if self.show_login_dialog {
            self.render_login_dialog(ctx);
        }

        if self.show_register_dialog {
            self.render_register_dialog(ctx);
        }

        if self.show_settings_dialog {
            self.render_settings_dialog(ctx);
        }

        // 底部状态栏
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Menu button
                if ui.button("☰").clicked() {
                    // Show menu options
                    ui.menu_button("登录", || {
                        self.show_login_dialog = true;
                        self.show_register_dialog = false;
                        self.show_settings_dialog = false;
                    });
                    if let Some(auth_client) = &self.auth_client {
                        match self.runtime.block_on(async {
                            auth_client.is_authenticated().await
                        }) {
                            Ok(_) => {
                                ui.menu_button("规则管理", || {
                                    self.show_rules_management_dialog(ctx);
                                });
                                ui.menu_button("报告管理", || {
                                    self.show_reports_management_dialog(ctx);
                                });
                                ui.menu_button("退出登录", || {
                                    match self.runtime.block_on(async {
                                        auth_client.logout().await
                                    }) {
                                        Ok(_) => {
                                            info!("User logged out successfully");
                                        }
                                        Err(e) => {
                                            error!("Logout failed: {}", e);
                                        }
                                    }
                                });
                            }
                            Err(_) => {
                                ui.menu_button("登录", || {
                                    self.show_login_dialog = true;
                                    self.show_register_dialog = false;
                                    self.show_settings_dialog = false;
                                });
                            }
                        }
                    });
                }

                // Authentication status indicator
                if let Some(auth_client) = &self.auth_client {
                    match self.runtime.block_on(async {
                        auth_client.is_authenticated().await
                    }) {
                        Ok(true) => {
                            ui.label(egui::RichText::new("●").color(egui::Color32::GREEN));
                            ui.label(self.localization.get("logged_in"));
                        }
                        Ok(false) => {
                            ui.label(egui::RichText::new("○").color(egui::Color32::RED));
                            ui.label(self.localization.get("not_logged_in"));
                        }
                        Err(_) => {
                            ui.label(egui::RichText::new("?").color(egui::Color32::YELLOW));
                            ui.label(self.localization.get("auth_error"));
                        }
                    }
                } else {
                    ui.label(egui::RichText::new("?").color(egui::Color32::YELLOW));
                    ui.label(self.localization.get("auth_not_initialized"));
                }

                ui.add_space(20.0);

                if self.scan_in_progress {
                    ui.spinner();

                    // 基本进度百分比
                    ui.label(format!("{}... {}%",
                        self.localization.get("scanning"),
                        (self.scan_progress * 100.0) as i32));

                    // 添加分隔符
                    ui.separator();

                    // 显示已扫描文件数
                    ui.label(format!("{} {}",
                        self.localization.get("files_scanned"),
                        self.files_scanned));

                    // 显示匹配规则数
                    ui.label(format!("{} {}",
                        self.localization.get("rules_matched"),
                        self.rules_matched));

                    // 显示已用时间
                    if let Some(start_time) = self.scan_start_time {
                        let elapsed = start_time.elapsed();
                        ui.label(format!("{} {:.1}s",
                            self.localization.get("time_elapsed"),
                            elapsed.as_secs_f32()));
                    }

                    // 显示详细扫描信息（如果有结果）
                    if let Some(results) = &self.scan_results {
                        // 漏洞统计
                        let total_vulns = results.stats.critical + results.stats.high +
                                         results.stats.medium + results.stats.low;

                        if total_vulns > 0 {
                            ui.separator();
                            if results.stats.critical > 0 {
                                ui.colored_label(ui::helpers::severity_color("critical"),
                                    format!("Critical: {}", results.stats.critical));
                            }
                            if results.stats.high > 0 {
                                ui.colored_label(ui::helpers::severity_color("high"),
                                    format!("High: {}", results.stats.high));
                            }
                            if results.stats.medium > 0 {
                                ui.colored_label(ui::helpers::severity_color("medium"),
                                    format!("Medium: {}", results.stats.medium));
                            }
                            if results.stats.low > 0 {
                                ui.colored_label(ui::helpers::severity_color("low"),
                                    format!("Low: {}", results.stats.low));
                            }
                        }
                    }
                } else if self.scan_results.is_some() {
                    let results = self.scan_results.as_ref().unwrap();
                    let total_vulns = results.stats.critical + results.stats.high +
                                     results.stats.medium + results.stats.low;

                    ui.label(format!("扫描完成 - 发现 {} 个漏洞", total_vulns));

                    // 按严重性分类
                    if total_vulns > 0 {
                        ui.separator();
                        if results.stats.critical > 0 {
                            ui.colored_label(ui::helpers::severity_color("critical"),
                                format!("严重: {}", results.stats.critical));
                        }
                        if results.stats.high > 0 {
                            ui.colored_label(ui::helpers::severity_color("high"),
                                format!("高危: {}", results.stats.high));
                        }
                        if results.stats.medium > 0 {
                            ui.colored_label(ui::helpers::severity_color("medium"),
                                format!("中危: {}", results.stats.medium));
                        }
                        if results.stats.low > 0 {
                            ui.colored_label(ui::helpers::severity_color("low"),
                                format!("低危: {}", results.stats.low));
                        }
                    }

                    // 显示扫描时间
                    ui.separator();
                    ui.label(format!("扫描时间: {:.1}s", results.stats.scan_time_ms as f32 / 1000.0));

                    // 显示上传状态
                    if self.uploading_report {
                        ui.separator();
                        ui.spinner();
                        ui.label("正在上传报告...");
                    } else if let Some(status) = &self.upload_status {
                        ui.separator();
                        ui.label(status);
                    }
                } else {
                    ui.label("就绪");
                }
            });
        });
    }
}

impl ScannerApp {
    fn new(cc: &CreationContext) -> Self {
        let mut app = Self::default();

        // 设置UI状态
        app.ui_state.server_url = app.config_manager.config().server.base_url.clone();

        // 根据配置设置UI主题
        if app.config_manager.config().ui.dark_mode {
            app.ui_state.dark_mode = true;
            cc.egui_ctx.set_visuals(Visuals::dark());
        } else {
            app.ui_state.dark_mode = false;
            cc.egui_ctx.set_visuals(Visuals::light());
        }

        // 配置字体，确保支持中文
        let mut fonts = egui::FontDefinitions::default();

        // 添加系统字体
        #[cfg(target_os = "macos")]
        {
            // 尝试加载系统中的中文字体
            let font_paths = [
                "/System/Library/Fonts/PingFang.ttc",
                "/Library/Fonts/Arial Unicode.ttf",
                "/System/Library/Fonts/STHeiti Light.ttc",
                "/System/Library/Fonts/STHeiti Medium.ttc",
            ];

            for path in &font_paths {
                if let Ok(font_data) = std::fs::read(path) {
                    fonts.font_data.insert(
                        "chinese_font".to_owned(),
                        egui::FontData::from_owned(font_data),
                    );

                    // 将中文字体添加到所有字体族中
                    for family in fonts.families.values_mut() {
                        family.insert(0, "chinese_font".to_owned());
                    }

                    break;
                }
            }
        }

        // 应用字体配置
        cc.egui_ctx.set_fonts(fonts);

        // 初始化规则管理器
        let rule_server_url = format!("{}/rules", app.config_manager.config().server.base_url);
        let mut rule_manager = rules::RuleManager::new(&rule_server_url);

        // 尝试加载默认规则
        if let Err(e) = rule_manager.load_default_rules() {
            error!("Failed to load default rules: {}", e);
        } else {
            app.rules_loaded = true;
        }

        app.rule_manager = Some(rule_manager);

        // 如果启用了自动更新规则，检查是否需要更新
        if app.config_manager.config().server.auto_update_rules {
            app.check_and_update_rules_if_needed();
        }

        info!("Application initialized with Tokio runtime and default rules");
        app
    }

    fn ui_scan_config(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.localization.get("tab_scan_config"));

        ui.horizontal(|ui| {
            ui.label(self.localization.get("target_path"));
            if ui.button(self.localization.get("select_directory")).clicked() {
                #[cfg(feature = "file_dialog")]
                {
                    // 设置默认目录为当前选择的目录或配置中的默认目录
                    let mut dialog = rfd::FileDialog::new();
                    if let Some(current_path) = &self.scan_path {
                        dialog = dialog.set_directory(current_path);
                    } else if let Some(default_dir) = &self.config_manager.config().paths.default_scan_dir {
                        dialog = dialog.set_directory(default_dir);
                    }

                    if let Some(path) = dialog.pick_folder() {
                        self.scan_path = Some(path.clone());

                        // 保存为默认扫描目录
                        self.config_manager.config_mut().paths.default_scan_dir = Some(path.clone());

                        // 添加到最近扫描列表
                        self.add_to_recent_scans(path);

                        // 保存配置
                        if let Err(e) = self.config_manager.save() {
                            warn!("Failed to save scan directory to config: {}", e);
                        }
                    }
                }
                #[cfg(not(feature = "file_dialog"))]
                {
                    info!("File dialog feature not enabled");
                    // In a real app, we would provide an alternative method here
                }
            }
        });

        if let Some(path) = &self.scan_path {
            ui.label(format!("{} {}", self.localization.get("selected"), path.display()));
        } else {
            ui.label("未选择扫描目录");
        }

        // 显示最近扫描的目录
        let recent_scans = self.config_manager.config().paths.recent_scans.clone();
        if !recent_scans.is_empty() {
            ui.separator();
            ui.label("最近扫描的目录:");

            // 使用滚动区域显示最近的目录
            egui::ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
                    for (i, recent_path) in recent_scans.iter().enumerate() {
                        if i >= 5 { break; } // 只显示最近的5个

                        ui.horizontal(|ui| {
                            // 显示目录路径（截断长路径）
                            let path_str = recent_path.display().to_string();
                            let display_path = if path_str.len() > 50 {
                                format!("...{}", &path_str[path_str.len()-47..])
                            } else {
                                path_str
                            };

                            if ui.small_button(&display_path).clicked() {
                                self.scan_path = Some(recent_path.clone());

                                // 更新默认扫描目录
                                self.config_manager.config_mut().paths.default_scan_dir = Some(recent_path.clone());

                                // 移动到列表顶部
                                self.add_to_recent_scans(recent_path.clone());

                                // 保存配置
                                if let Err(e) = self.config_manager.save() {
                                    warn!("Failed to save recent scan selection: {}", e);
                                }
                            }

                            // 显示完整路径的工具提示
                            if ui.small_button("📋").on_hover_text("复制路径").clicked() {
                                ui.output_mut(|o| o.copied_text = recent_path.display().to_string());
                            }
                        });
                    }
                });
        }

        ui.separator();
        ui.heading(self.localization.get("scanning_rules"));

        // 显示规则信息
        if self.downloading_rules {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("正在下载最新规则...");
            });
        } else if let Some(error) = &self.download_error {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::RED, format!("规则下载失败: {}", error));
            });
            if ui.button("重试").clicked() {
                self.download_error = None;
                self.download_rules();
            }
        } else if self.rules_loaded {
            if let Some(rule_manager) = &self.rule_manager {
                let ruleset = rule_manager.ruleset();
                let stats = ruleset.stats();

                ui.label(format!("规则版本: {}", ruleset.version));
                ui.label(format!("文件类型: {}", stats.file_types));
                ui.label(format!("总规则数: {}", stats.patterns));

                ui.collapsing("按严重性分类的漏洞模式", |ui| {
                    ui.label(format!("严重: {}", stats.critical));
                    ui.label(format!("高危: {}", stats.high));
                    ui.label(format!("中危: {}", stats.medium));
                    ui.label(format!("低危: {}", stats.low));
                });

                // 显示文件类型和规则数量
                ui.collapsing("支持的文件类型", |ui| {
                    for file_type in &ruleset.file_types {
                        ui.label(format!("{}: {} 个规则", file_type.name, file_type.patterns.len()));
                    }
                });

                if ui.button(self.localization.get("update_rules")).clicked() && !self.downloading_rules {
                    self.download_latest_rules();
                }
            }
        } else {
            ui.label(self.localization.get("no_rules_loaded"));
            if ui.button(self.localization.get("download_rules")).clicked() {
                self.download_latest_rules();
            }
        }

        ui.separator();

        // 规则更新设置
        ui.heading("规则设置");
        let mut auto_update_rules = self.config_manager.config().server.auto_update_rules;
        if ui.checkbox(&mut auto_update_rules, "自动检查并更新扫描规则").changed() {
            self.config_manager.config_mut().server.auto_update_rules = auto_update_rules;
            if let Err(e) = self.config_manager.save() {
                warn!("Failed to save auto update rules setting: {}", e);
            }
        }

        // 显示最后更新时间
        if let Some(last_update) = &self.config_manager.config().server.last_rule_update {
            ui.label(format!("最后更新: {}", last_update));
        } else {
            ui.label("规则尚未更新");
        }

        ui.separator();

        // 自动上传扫描报告选项
        ui.heading("上传设置");
        let mut auto_upload = self.config_manager.config().server.auto_upload_reports;
        if ui.checkbox(&mut auto_upload, "自动上传扫描报告到服务器").changed() {
            self.config_manager.config_mut().server.auto_upload_reports = auto_upload;
            if let Err(e) = self.config_manager.save() {
                warn!("Failed to save auto upload setting: {}", e);
            }
        }

        ui.separator();

        // 只有在有扫描规则和选择了路径的情况下才启用扫描按钮
        if self.scan_path.is_some() && self.rules_loaded {
            if ui.button(self.localization.get("start_scan")).clicked() {
                self.start_scan();
            }
        } else {
            // 禁用状态的按钮
            let btn = egui::Button::new(self.localization.get("start_scan"));
            ui.add_enabled(false, btn);
            if self.scan_path.is_none() {
                ui.label(self.localization.get("please_select_directory"));
            }
            if !self.rules_loaded {
                ui.label(self.localization.get("please_load_rules"));
            }
        }
    }

    fn ui_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Real-time Dashboard");

        if self.scan_in_progress {
            ui.horizontal(|ui| {
                // Left side: Progress statistics
                ui.vertical(|ui| {
                    ui.heading("Scan Progress");
                    ui.label(format!("Files scanned: {}", self.scan_results.as_ref().map_or(0, |r| r.stats.scanned_files)));
                    ui.label(format!("Vulnerabilities found: {}", self.scan_results.as_ref().map_or(0, |r| {
                        r.stats.critical + r.stats.high + r.stats.medium + r.stats.low
                    })));
                    ui.add_space(20.0);

                    // Circular Progress Chart
                    let progress = self.scan_progress;
                    let mut progress_chart = visualization::CircularProgress::new(progress)
                        .radius(60.0)
                        .thickness(10.0)
                        .label(format!("{}%", (progress * 100.0) as i32));
                    progress_chart.render(ui);
                });

                // Right side: Real-time 3D heatmap (if we have results)
                if let Some(results) = &self.scan_results {
                    if !results.hotspots.is_empty() {
                        ui.vertical(|ui| {
                            ui.heading("Vulnerability Heatmap");
                            ui.label("Click on a hotspot to see details.");

                            // Initialize heatmap if not already done
                            if self.heatmap.is_none() {
                                self.heatmap = Some(visualization::Heatmap3D::new());
                            }

                            if let Some(heatmap) = &mut self.heatmap {
                                heatmap.update_hotspots(&results.hotspots);
                                if let Some((file_path, rule_id)) = heatmap.render(ui) {
                                    // Handle click on hotspot - switch to details view with this file+rule
                                    self.selected_hotspot = Some((file_path, rule_id));
                                    self.ui_state.active_tab = ui::ActiveTab::VulnerabilityDetails;
                                }
                            }
                        });
                    }
                }
            });
        } else if let Some(results) = &self.scan_results {
            ui.horizontal(|ui| {
                // Left side: Statistics
                ui.vertical(|ui| {
                    ui.heading("Scan Results");
                    ui.label(format!("Total vulnerabilities found: {}",
                        results.stats.critical + results.stats.high + results.stats.medium + results.stats.low));

                    // Display statistics in a formatted way
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.colored_label(ui::helpers::severity_color("critical"),
                                    format!("Critical: {}", results.stats.critical));
                                ui.colored_label(ui::helpers::severity_color("high"),
                                    format!("High: {}", results.stats.high));
                            });
                            ui.vertical(|ui| {
                                ui.colored_label(ui::helpers::severity_color("medium"),
                                    format!("Medium: {}", results.stats.medium));
                                ui.colored_label(ui::helpers::severity_color("low"),
                                    format!("Low: {}", results.stats.low));
                            });
                        });
                    });

                    ui.add_space(10.0);
                    ui.label(format!("Files scanned: {}", results.stats.scanned_files));
                    ui.label(format!("Scan time: {} ms", results.stats.scan_time_ms));

                    if ui.button("View Detailed Report").clicked() {
                        self.ui_state.active_tab = ui::ActiveTab::VulnerabilityDetails;
                    }
                });

                // Right side: 3D heatmap visualization
                ui.vertical(|ui| {
                    ui.heading("Vulnerability Heatmap");
                    ui.label("Click on a hotspot to see details.");

                    // Initialize heatmap if not already done
                    if self.heatmap.is_none() {
                        self.heatmap = Some(visualization::Heatmap3D::new());
                    }

                    if let Some(heatmap) = &mut self.heatmap {
                        heatmap.update_hotspots(&results.hotspots);
                        if let Some((file_path, rule_id)) = heatmap.render(ui) {
                            // Handle click on hotspot - switch to details view with this file+rule
                            self.selected_hotspot = Some((file_path, rule_id));
                            self.ui_state.active_tab = ui::ActiveTab::VulnerabilityDetails;
                        }
                    }
                });
            });
        } else {
            ui.label("No scan results available. Start a scan to see results here.");

            // Show empty/demo visualization
            if self.heatmap.is_none() {
                self.heatmap = Some(visualization::Heatmap3D::new());
            }

            ui.add_space(20.0);
            ui.label("3D Heatmap Preview (No Data)");
            if let Some(heatmap) = &mut self.heatmap {
                heatmap.render(ui);
            }
        }
    }

    fn ui_vuln_details(&mut self, ui: &mut egui::Ui) {
        ui.heading("Vulnerability Details");

        if let Some(results) = &self.scan_results {
            // 显示漏洞统计信息
            let total_vulns = results.stats.critical + results.stats.high + results.stats.medium + results.stats.low;
            ui.label(format!("总计发现 {} 个漏洞", total_vulns));

            ui.horizontal(|ui| {
                if results.stats.critical > 0 {
                    ui.colored_label(ui::helpers::severity_color("critical"),
                        format!("严重: {}", results.stats.critical));
                }
                if results.stats.high > 0 {
                    ui.colored_label(ui::helpers::severity_color("high"),
                        format!("高危: {}", results.stats.high));
                }
                if results.stats.medium > 0 {
                    ui.colored_label(ui::helpers::severity_color("medium"),
                        format!("中危: {}", results.stats.medium));
                }
                if results.stats.low > 0 {
                    ui.colored_label(ui::helpers::severity_color("low"),
                        format!("低危: {}", results.stats.low));
                }
            });

            ui.separator();

            // 使用ScrollArea包装漏洞详情列表，添加滚动条
            egui::ScrollArea::vertical()
                .max_height(ui.available_height() - 20.0) // 留出一些底部空间
                .auto_shrink([false; 2]) // 不自动收缩
                .show(ui, |ui| {
                    // 显示漏洞详情列表
                    for (i, snippet) in results.code_snippets.iter().enumerate() {
                        ui.collapsing(format!("{}: {} ({})", i+1, snippet.title, &snippet.severity.to_string()), |ui| {
                            ui.label(format!("文件: {}", snippet.file_path.display()));
                            ui.label(format!("行号: {}", snippet.line_number));

                            // 代码片段使用单独的滚动区域，以防代码过长
                            ui.label("代码片段:");
                            egui::ScrollArea::horizontal()
                                .max_height(150.0)
                                .show(ui, |ui| {
                                    ui.code(snippet.code.clone());
                                });

                            ui.label(format!("描述: {}", snippet.description));
                        });

                        // 在每个漏洞项之间添加一些间距
                        ui.add_space(5.0);
                    }
                });
        } else {
            ui.label("No vulnerabilities to display. Run a scan first.");
        }
    }

    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.localization.get("tab_settings"));

        // 主题设置
        ui.checkbox(&mut self.ui_state.dark_mode, self.localization.get("dark_mode"));
        if self.ui_state.dark_mode_changed {
            if self.ui_state.dark_mode {
                ui.ctx().set_visuals(Visuals::dark());
            } else {
                ui.ctx().set_visuals(Visuals::light());
            }
            self.ui_state.dark_mode_changed = false;

            // 更新配置
            self.config_manager.config_mut().ui.dark_mode = self.ui_state.dark_mode;
        }

        ui.separator();
        ui.heading("服务器设置");

        // 服务器基础URL
        let mut base_url = self.config_manager.config().server.base_url.clone();
        ui.horizontal(|ui| {
            ui.label(self.localization.get("base_url"));
            if ui.text_edit_singleline(&mut base_url).changed() {
                self.config_manager.config_mut().server.base_url = base_url.clone();

                // 更新派生的URL
                self.config_manager.config_mut().server.rule_server_url = format!("{}/rules", base_url);
                self.config_manager.config_mut().server.report_server_url = format!("{}/reports/upload", base_url);
            }
        });

        // 用户名
        let mut username = self.config_manager.config().server.username.clone();
        ui.horizontal(|ui| {
            ui.label(self.localization.get("username"));
            if ui.text_edit_singleline(&mut username).changed() {
                self.config_manager.config_mut().server.username = username;
            }
        });

        // 密码
        let mut password = self.config_manager.config().server.password.clone();
        ui.horizontal(|ui| {
            ui.label(self.localization.get("password"));
            if ui.text_edit_singleline(&mut password).changed() {
                self.config_manager.config_mut().server.password = password;
            }
        });

        // 保存按钮
        if ui.button(self.localization.get("save_settings")).clicked() {
            if let Err(e) = self.config_manager.save() {
                error!("Failed to save configuration: {}", e);
            } else {
                info!("Configuration saved successfully");
            }
        }

        // 扫描历史
        ui.separator();
        ui.heading(self.localization.get("scan_history"));

        if self.config_manager.config().server.scan_history.is_empty() {
            ui.label("暂无扫描历史记录");
        } else {
            // 显示扫描历史
            let history = self.config_manager.config().server.scan_history.clone();
            for (i, entry) in history.iter().enumerate() {
                let timestamp = entry.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
                let target = entry.target_path.display().to_string();

                let status = match &entry.upload_status {
                    config::UploadStatus::NotUploaded => "未上传".to_string(),
                    config::UploadStatus::Uploading => "上传中".to_string(),
                    config::UploadStatus::Uploaded => "已上传".to_string(),
                    config::UploadStatus::Failed(err) => format!("上传失败: {}", err),
                };

                let report_id_clone = entry.server_response.clone();
                let report_path_clone = entry.report_path.clone();
                let is_uploadable = matches!(entry.upload_status, config::UploadStatus::NotUploaded | config::UploadStatus::Failed(_));

                ui.collapsing(format!("#{} - {} - {}", i+1, timestamp, target), |ui| {
                    ui.label(format!("上传状态: {}", status));

                    if let Some(report_id) = &report_id_clone {
                        ui.label(format!("报告ID: {}", report_id));
                    }

                    if let Some(report_path) = &report_path_clone {
                        ui.label(format!("报告文件: {}", report_path.display()));
                    }

                    // 如果报告未上传，显示上传按钮
                    if is_uploadable {
                        if ui.button(self.localization.get("upload_report")).clicked() {
                            // 打开上传对话框
                            self.ui_state.show_upload_dialog = true;
                            self.ui_state.upload_dialog_data.entry_index = i;
                            self.ui_state.upload_dialog_data.report_path = report_path_clone.clone();

                            // 预填充规则版本（如果有）
                            if let Some(rule_manager) = &self.rule_manager {
                                self.ui_state.upload_dialog_data.rule_version = rule_manager.ruleset().version.clone();
                            }
                        }
                    }
                });
            }
        }
    }

    // 下载最新规则（从服务器的/rules/latest端点）
    fn download_latest_rules(&mut self) {
        if self.downloading_rules {
            return; // 已经在下载中
        }

        self.downloading_rules = true;
        self.download_error = None;

        // 获取服务器基础URL
        let base_url = self.config_manager.config().server.base_url.clone();
        let rules_url = format!("{}/rules/latest", base_url);

        info!("Downloading latest rules from {}", rules_url);

        // 创建临时目录用于存储下载的规则
        let rules_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("./cache"))
            .join("sdchat-scanner/rules");

        if !rules_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&rules_dir) {
                error!("Failed to create rules directory: {}", e);
                self.downloading_rules = false;
                self.download_error = Some(format!("Failed to create rules directory: {}", e));
                return;
            }
        }

        let rules_file = rules_dir.join("latest_rules.json");

        // 克隆需要的数据以便在异步闭包中使用
        let rules_file_clone = rules_file.clone();

        // 使用Tokio运行时执行异步下载
        let handle = self.runtime.spawn(async move {
            // 创建HTTP客户端
            let client = reqwest::Client::new();

            // 发送GET请求
            let response = match client.get(&rules_url).send().await {
                Ok(resp) => resp,
                Err(e) => return Err(anyhow::anyhow!("HTTP request failed: {}", e)),
            };

            // 检查响应状态
            if !response.status().is_success() {
                return Err(anyhow::anyhow!("Server returned error: {}", response.status()));
            }

            // 读取响应内容
            let rules_json = match response.text().await {
                Ok(text) => text,
                Err(e) => return Err(anyhow::anyhow!("Failed to read response: {}", e)),
            };

            // 将规则保存到文件
            match tokio::fs::write(&rules_file_clone, rules_json).await {
                Ok(_) => Ok(rules_file_clone),
                Err(e) => Err(anyhow::anyhow!("Failed to write rules file: {}", e)),
            }
        });

        // 阻塞等待下载完成
        match self.runtime.block_on(handle) {
            Ok(result) => {
                match result {
                    Ok(file_path) => {
                        info!("Rules downloaded to {}", file_path.display());

                        // 加载下载的规则
                        if let Some(rule_manager) = &mut self.rule_manager {
                            match rule_manager.load_rules_from_file(&file_path) {
                                Ok(_) => {
                                    info!("Rules loaded successfully");
                                    self.rules_loaded = true;

                                    // 更新配置中的最后更新时间
                                    let now = chrono::Utc::now().to_rfc3339();
                                    self.config_manager.config_mut().server.last_rule_update = Some(now);

                                    // 保存配置
                                    if let Err(e) = self.config_manager.save() {
                                        warn!("Failed to save configuration: {}", e);
                                    }
                                },
                                Err(e) => {
                                    error!("Failed to load rules: {}", e);
                                    self.download_error = Some(format!("Failed to load rules: {}", e));
                                }
                            }
                        }
                    },
                    Err(e) => {
                        error!("Failed to download rules: {}", e);
                        self.download_error = Some(format!("Failed to download rules: {}", e));
                    }
                }
            },
            Err(e) => {
                error!("Tokio task failed: {}", e);
                self.download_error = Some(format!("Tokio task failed: {}", e));
            }
        }

        self.downloading_rules = false;
    }

    // 原有的下载规则方法（保留向后兼容性）
    fn download_rules(&mut self) {
        if self.downloading_rules {
            return; // 已经在下载中
        }

        self.downloading_rules = true;
        self.download_error = None;

        // 获取规则管理器的引用
        if let Some(rule_manager) = &self.rule_manager {
            // 创建规则管理器克隆
            let mut rule_manager_clone = rule_manager.clone();

            // 获取当前UI上下文引用，用于请求重绘
            let ctx = egui::Context::clone(&egui::Context::default());

            // 使用tokio运行时直接执行异步任务
            let handle = self.runtime.spawn(async move {
                // 执行规则更新
                match rule_manager_clone.update_rules().await {
                    Ok(_) => {
                        info!("Rules updated successfully");
                        Ok(rule_manager_clone.ruleset().clone())
                    },
                    Err(e) => {
                        let error_msg = format!("{}", e);
                        error!("Failed to update rules: {}", error_msg);
                        Err(error_msg)
                    }
                }
            });

            // 阻塞等待任务完成（因为这是按钮点击响应，短暂阻塞是可接受的）
            match self.runtime.block_on(handle) {
                Ok(result) => {
                    self.downloading_rules = false;

                    match result {
                        Ok(ruleset) => {
                            // 更新规则集
                            if let Some(manager) = &mut self.rule_manager {
                                manager.update_ruleset(ruleset);
                                self.rules_loaded = true;
                            }
                        },
                        Err(error) => {
                            // 设置错误状态
                            self.download_error = Some(error);
                        }
                    }
                },
                Err(e) => {
                    error!("Task join error: {}", e);
                    self.downloading_rules = false;
                    self.download_error = Some(format!("Task error: {}", e));
                }
            }
        } else {
            // 没有规则管理器，直接设置错误
            self.downloading_rules = false;
            self.download_error = Some("Rule manager not initialized".to_string());
        }
    }

    fn start_scan(&mut self) {
        // 确保已选择路径
        let path = match &self.scan_path {
            Some(p) => p.clone(),
            None => {
                warn!("Cannot start scan: No path selected");
                return;
            }
        };

        // 确保规则已加载
        if !self.rules_loaded {
            warn!("Cannot start scan: Rules not loaded");
            return;
        }

        // 初始化状态
        self.scan_in_progress = true;
        self.scan_progress = 0.0;
        self.scan_results = None; // 清除之前的结果
        self.files_scanned = 0;
        self.rules_matched = 0;
        self.scan_start_time = Some(std::time::Instant::now());

        info!("Starting scan of path: {:?}", path);

        // 获取规则集
        let rules = match &self.rule_manager {
            Some(manager) => manager.ruleset().clone(),
            None => {
                warn!("Rule manager not initialized, using default rules");
                rules::RuleSet::default()
            }
        };

        // 创建扫描配置
        let mut ignore_patterns = Vec::new();

        // 从配置中添加忽略的文件扩展名
        for ext in &self.config_manager.config().scanner.ignored_extensions {
            if let Ok(pattern) = regex::Regex::new(&format!(r"\.{}$", ext)) {
                ignore_patterns.push(pattern);
            }
        }

        // 从配置中添加忽略的目录
        for dir in &self.config_manager.config().scanner.ignored_directories {
            if let Ok(pattern) = regex::Regex::new(&format!(r"/{}/", dir)) {
                ignore_patterns.push(pattern);
            }
        }

        let config = scanner::ScanConfig {
            target_path: path,
            rules,
            thread_count: self.config_manager.config().scanner.thread_count,
            max_file_size: self.config_manager.config().scanner.max_file_size,
            ignore_patterns,
        };

        // 创建扫描器并存储在应用状态中
        let scanner = Arc::new(scanner::Scanner::new(config));
        self.scanner = Some(scanner.clone());

        // 使用Tokio运行时启动异步扫描
        let handle = self.runtime.spawn(async move {
            scanner.start_scan_async().await
        });

        // 阻塞等待扫描启动（获取进度通道）
        match self.runtime.block_on(handle) {
            Ok(result) => {
                match result {
                    Ok(progress_rx) => {
                        // 存储进度接收器以便在UI更新中使用
                        self.progress_rx = Some(progress_rx);
                        info!("Scan progress channel established");
                    },
                    Err(e) => {
                        error!("Failed to start scan: {}", e);
                        self.scan_in_progress = false;
                    }
                }
            },
            Err(e) => {
                error!("Tokio task failed: {}", e);
                self.scan_in_progress = false;
            }
        }
    }

    // 扫描完成后处理结果
    fn handle_scan_completed(&mut self) {
        if let Some(results) = &self.scan_results {
            info!("Scan completed with {} vulnerabilities",
                results.stats.critical + results.stats.high + results.stats.medium + results.stats.low);

            // 生成报告
            if let Some(scan_path) = &self.scan_path {
                let report_generator = report::ReportGenerator::new(results.clone(), scan_path.clone());

                // 生成JSON报告
                match report_generator.generate(report::ReportFormat::Json) {
                    Ok(report_path) => {
                        info!("Report generated at {}", report_path.display());

                        // 添加到扫描历史
                        let history_entry = config::ScanHistoryEntry {
                            timestamp: chrono::Utc::now(),
                            target_path: scan_path.clone(),
                            report_path: Some(report_path.clone()),
                            upload_status: config::UploadStatus::NotUploaded,
                            server_response: None,
                        };

                        self.config_manager.config_mut().server.scan_history.insert(0, history_entry);

                        // 保存配置
                        if let Err(e) = self.config_manager.save() {
                            error!("Failed to save scan history: {}", e);
                        }

                        // 检查是否启用了自动上传
                        if self.config_manager.config().server.auto_upload_reports {
                            info!("Auto-upload is enabled, uploading report automatically");

                            // 获取默认元数据
                            let application_name = "自动扫描".to_string();
                            let uploader_name = "系统".to_string();
                            let rule_version = if let Some(rule_manager) = &self.rule_manager {
                                rule_manager.ruleset().version.clone()
                            } else {
                                "未知".to_string()
                            };
                            let notes = "自动上传的扫描报告".to_string();

                            // 自动上传报告
                            let base_url = self.config_manager.config().server.base_url.clone();
                            let username = self.config_manager.config().server.username.clone();
                            let password = self.config_manager.config().server.password.clone();

                            self.upload_report_with_metadata(
                                0, // 最新的历史记录索引
                                Some(report_path.clone()),
                                &base_url,
                                &username,
                                &password,
                                application_name,
                                uploader_name,
                                rule_version,
                                notes
                            );
                        } else {
                            info!("Auto-upload is disabled. Report generated and ready for manual upload.");
                            self.upload_status = Some("报告已生成，可以在仪表盘中手动上传".to_string());
                        }
                    },
                    Err(e) => {
                        error!("Failed to generate report: {}", e);
                    }
                }
            }
        }
    }

    // 获取JWT令牌
    async fn get_jwt_token(client: &reqwest::Client, base_url: &str, username: &str, password: &str) -> Result<String, anyhow::Error> {
        // 构建登录URL
        let login_url = format!("{}/auth/login", base_url);

        // 准备登录数据
        let login_data = serde_json::json!({
            "username": username,
            "password": password
        });

        // 发送登录请求
        let response = client.post(&login_url)
            .header("Content-Type", "application/json")
            .json(&login_data)
            .send()
            .await?;

        // 检查响应状态
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Login failed: {}", response.status()));
        }

        // 解析响应获取令牌
        let response_json: serde_json::Value = response.json().await?;

        // 提取访问令牌
        match response_json.get("access_token") {
            Some(token) => {
                if let Some(token_str) = token.as_str() {
                    Ok(token_str.to_string())
                } else {
                    Err(anyhow::anyhow!("Invalid token format"))
                }
            },
            None => Err(anyhow::anyhow!("No access token in response"))
        }
    }

    // 渲染上传对话框
    fn render_upload_dialog(&mut self, ctx: &Context) {
        let mut open = self.ui_state.show_upload_dialog;

        egui::Window::new("上传扫描报告")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .min_width(400.0)
            .show(ctx, |ui| {
                ui.heading("填写报告元数据");
                ui.add_space(10.0);

                // 归属应用
                ui.horizontal(|ui| {
                    ui.label("归属应用:");
                    ui.text_edit_singleline(&mut self.ui_state.upload_dialog_data.application_name);
                });

                // 上传人
                ui.horizontal(|ui| {
                    ui.label("上传人:");
                    ui.text_edit_singleline(&mut self.ui_state.upload_dialog_data.uploader_name);
                });

                // 扫描规则版本
                ui.horizontal(|ui| {
                    ui.label("规则版本:");
                    ui.text_edit_singleline(&mut self.ui_state.upload_dialog_data.rule_version);
                });

                // 备注
                ui.horizontal(|ui| {
                    ui.label("备注信息:");
                    ui.text_edit_multiline(&mut self.ui_state.upload_dialog_data.notes);
                });

                ui.add_space(10.0);

                // 按钮区域
                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() {
                        self.ui_state.show_upload_dialog = false;
                    }

                    if ui.button("上传").clicked() {
                        // 获取上传所需的数据
                        let entry_index = self.ui_state.upload_dialog_data.entry_index;
                        let report_path = self.ui_state.upload_dialog_data.report_path.clone();
                        let base_url = self.config_manager.config().server.base_url.clone();
                        let username = self.config_manager.config().server.username.clone();
                        let password = self.config_manager.config().server.password.clone();

                        // 关闭对话框
                        self.ui_state.show_upload_dialog = false;

                        // 开始上传
                        self.upload_report_with_metadata(
                            entry_index,
                            report_path,
                            &base_url,
                            &username,
                            &password,
                            self.ui_state.upload_dialog_data.application_name.clone(),
                            self.ui_state.upload_dialog_data.uploader_name.clone(),
                            self.ui_state.upload_dialog_data.rule_version.clone(),
                            self.ui_state.upload_dialog_data.notes.clone()
                        );
                    }
                });
            });

        // 如果对话框被关闭，更新状态
        if !open {
            self.ui_state.show_upload_dialog = false;
        }
    }

    // 上传报告到服务器（带元数据）
    fn upload_report_with_metadata(
        &mut self,
        entry_index: usize,
        report_path: Option<PathBuf>,
        base_url: &str,
        username: &str,
        password: &str,
        application_name: String,
        uploader_name: String,
        rule_version: String,
        notes: String
    ) {
        if self.uploading_report {
            return; // 已经在上传中
        }

        // 检查报告路径
        let report_path = match report_path {
            Some(path) => {
                if !path.exists() {
                    error!("Report file does not exist: {}", path.display());
                    self.upload_status = Some("报告文件不存在".to_string());
                    return;
                }
                path
            },
            None => {
                error!("No report file specified");
                self.upload_status = Some("未指定报告文件".to_string());
                return;
            }
        };

        self.uploading_report = true;
        self.upload_status = Some("正在上传报告...".to_string());

        // 更新历史记录状态
        if entry_index < self.config_manager.config_mut().server.scan_history.len() {
            self.config_manager.config_mut().server.scan_history[entry_index].upload_status =
                config::UploadStatus::Uploading;

            if let Err(e) = self.config_manager.save() {
                warn!("Failed to save upload status: {}", e);
            }
        }

        // 构建上传URL
        let upload_url = format!("{}/reports/upload", base_url);

        // 克隆需要的数据以便在异步闭包中使用
        let report_path_clone = report_path.clone();
        let entry_index_clone = entry_index;
        let config_manager = Arc::new(std::sync::Mutex::new(self.config_manager.clone()));
        let username = username.to_string(); // 克隆字符串而不是使用引用
        let password = password.to_string(); // 克隆字符串而不是使用引用
        let base_url = base_url.to_string(); // 克隆基础URL

        // 克隆元数据
        let application_name = application_name.clone();
        let uploader_name = uploader_name.clone();
        let rule_version = rule_version.clone();
        let notes = notes.clone();

        // 使用Tokio运行时执行异步上传
        let handle = self.runtime.spawn(async move {
            // 读取报告文件
            let mut report_content: serde_json::Value = match tokio::fs::read_to_string(&report_path_clone).await {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(json) => json,
                    Err(e) => return Err(anyhow::anyhow!("Failed to parse report JSON: {}", e)),
                },
                Err(e) => return Err(anyhow::anyhow!("Failed to read report file: {}", e)),
            };

            // 添加元数据到报告
            if let Some(obj) = report_content.as_object_mut() {
                if !application_name.is_empty() {
                    obj.insert("application_name".to_string(), serde_json::Value::String(application_name));
                }
                if !uploader_name.is_empty() {
                    obj.insert("uploader_name".to_string(), serde_json::Value::String(uploader_name));
                }
                if !rule_version.is_empty() {
                    obj.insert("rule_version".to_string(), serde_json::Value::String(rule_version));
                }
                if !notes.is_empty() {
                    obj.insert("notes".to_string(), serde_json::Value::String(notes));
                }
            }

            // 将修改后的报告转换回字符串
            let report_json = match serde_json::to_string(&report_content) {
                Ok(json) => json,
                Err(e) => return Err(anyhow::anyhow!("Failed to serialize report: {}", e)),
            };

            // 创建HTTP客户端
            let client = reqwest::Client::new();

            // 首先获取JWT令牌
            info!("Getting JWT token for report upload");
            let token = match Self::get_jwt_token(&client, &base_url, &username, &password).await {
                Ok(token) => token,
                Err(e) => return Err(anyhow::anyhow!("Failed to get JWT token: {}", e)),
            };

            info!("Successfully obtained JWT token");

            // 使用JWT令牌发送POST请求
            let response = match client.post(&upload_url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(report_json)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => return Err(anyhow::anyhow!("HTTP request failed: {}", e)),
            };

            // 检查响应状态
            if !response.status().is_success() {
                return Err(anyhow::anyhow!("Server returned error: {}", response.status()));
            }

            // 读取响应内容
            let response_json = match response.text().await {
                Ok(text) => text,
                Err(e) => return Err(anyhow::anyhow!("Failed to read response: {}", e)),
            };

            // 解析响应JSON
            let response_data: serde_json::Value = match serde_json::from_str(&response_json) {
                Ok(data) => data,
                Err(e) => return Err(anyhow::anyhow!("Failed to parse response: {}", e)),
            };

            // 获取报告ID
            let report_id = match response_data.get("report_id") {
                Some(id) => id.as_str().unwrap_or("unknown").to_string(),
                None => "unknown".to_string(),
            };

            // 更新历史记录
            if let Ok(mut config_manager) = config_manager.lock() {
                if entry_index_clone < config_manager.config().server.scan_history.len() {
                    let mut config = config_manager.config_mut();
                    config.server.scan_history[entry_index_clone].upload_status =
                        config::UploadStatus::Uploaded;
                    config.server.scan_history[entry_index_clone].server_response = Some(report_id.clone());

                    if let Err(e) = config_manager.save() {
                        warn!("Failed to save upload status: {}", e);
                    }
                }
            }

            Ok(report_id)
        });

        // 阻塞等待上传完成
        match self.runtime.block_on(handle) {
            Ok(result) => {
                match result {
                    Ok(report_id) => {
                        info!("Report uploaded successfully, ID: {}", report_id);
                        self.upload_status = Some(format!("报告上传成功，ID: {}", report_id));
                    },
                    Err(e) => {
                        error!("Failed to upload report: {}", e);
                        self.upload_status = Some(format!("报告上传失败: {}", e));

                        // 更新历史记录状态
                        if entry_index < self.config_manager.config_mut().server.scan_history.len() {
                            self.config_manager.config_mut().server.scan_history[entry_index].upload_status =
                                config::UploadStatus::Failed(e.to_string());

                            if let Err(e) = self.config_manager.save() {
                                warn!("Failed to save upload status: {}", e);
                            }
                        }
                    }
                }
            },
            Err(e) => {
                error!("Tokio task failed: {}", e);
                self.upload_status = Some(format!("上传任务失败: {}", e));

                // 更新历史记录状态
                if entry_index < self.config_manager.config_mut().server.scan_history.len() {
                    self.config_manager.config_mut().server.scan_history[entry_index].upload_status =
                        config::UploadStatus::Failed(e.to_string());

                    if let Err(e) = self.config_manager.save() {
                        warn!("Failed to save upload status: {}", e);
                    }
                }
            }
        }

        self.uploading_report = false;
    }

    // 兼容性函数，保持原有调用方式不变
    fn upload_report(&mut self, entry_index: usize, report_path: Option<PathBuf>, base_url: &str, username: &str, password: &str) {
        // 调用新的带元数据的上传函数，但使用空字符串作为元数据
        self.upload_report_with_metadata(
            entry_index,
            report_path,
            base_url,
            username,
            password,
            String::new(),
            String::new(),
            String::new(),
            String::new()
        );
    }

    /// 添加路径到最近扫描列表
    fn add_to_recent_scans(&mut self, path: PathBuf) {
        let recent_scans = &mut self.config_manager.config_mut().paths.recent_scans;

        // 如果路径已存在，先移除它
        recent_scans.retain(|p| p != &path);

        // 添加到列表开头
        recent_scans.insert(0, path);

        // 限制列表长度为10个
        if recent_scans.len() > 10 {
            recent_scans.truncate(10);
        }
    }

    /// 检查并在需要时自动更新规则
    fn check_and_update_rules_if_needed(&mut self) {
        // 检查是否启用了自动更新
        if !self.config_manager.config().server.auto_update_rules {
            return;
        }

        // 检查最后更新时间
        let should_update = match &self.config_manager.config().server.last_rule_update {
            Some(last_update_str) => {
                // 解析最后更新时间
                match chrono::DateTime::parse_from_rfc3339(last_update_str) {
                    Ok(last_update) => {
                        let now = chrono::Utc::now();
                        let duration = now.signed_duration_since(last_update.with_timezone(&chrono::Utc));

                        // 如果超过24小时，则需要更新
                        duration.num_hours() >= 24
                    },
                    Err(_) => {
                        // 如果解析失败，认为需要更新
                        true
                    }
                }
            },
            None => {
                // 如果没有更新记录，需要更新
                true
            }
        };

        if should_update {
            info!("Auto-updating rules due to schedule");
            self.download_latest_rules();
        } else {
            info!("Rules are up to date, skipping auto-update");
        }
    }

    /// 定期检查规则更新（在UI更新循环中调用）
    fn periodic_rule_check(&mut self) {
        // 检查是否启用了自动更新
        if !self.config_manager.config().server.auto_update_rules {
            return;
        }

        // 如果正在下载规则，跳过检查
        if self.downloading_rules {
            return;
        }

        let now = std::time::Instant::now();

        // 检查是否需要进行定期检查（每小时检查一次）
        let should_check = match self.last_rule_check {
            Some(last_check) => {
                now.duration_since(last_check).as_secs() >= 3600 // 1小时 = 3600秒
            },
            None => true, // 如果从未检查过，立即检查
        };

        if should_check {
            self.last_rule_check = Some(now);

            // 检查是否需要更新规则
            let should_update = match &self.config_manager.config().server.last_rule_update {
                Some(last_update_str) => {
                    match chrono::DateTime::parse_from_rfc3339(last_update_str) {
                        Ok(last_update) => {
                            let now_utc = chrono::Utc::now();
                            let duration = now_utc.signed_duration_since(last_update.with_timezone(&chrono::Utc));

                            // 如果超过24小时，则需要更新
                            duration.num_hours() >= 24
                        },
                        Err(_) => true,
                    }
                },
                None => true,
            };

            if should_update {
                info!("Periodic rule check: updating rules");
                self.download_latest_rules();
            }
        }
    }
}

fn main() -> Result<()> {
    // 初始化日志
    env_logger::init();

    // Get version from package info
    let version = env!("CARGO_PKG_VERSION");

    info!("Starting {} v{}", APP_NAME_CN, version);

    #[cfg(feature = "hyperscan_engine")]
    info!("Using Hyperscan regex engine for pattern matching");

    #[cfg(not(feature = "hyperscan_engine"))]
    info!("Using standard Rust regex engine for pattern matching");

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0]),
        vsync: true,
        hardware_acceleration: eframe::HardwareAcceleration::Preferred,
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME_CN,
        options,
        Box::new(|cc| Box::new(ScannerApp::new(cc)))
    ).map_err(|e| anyhow::anyhow!("Application error: {}", e))?;

    Ok(())
}