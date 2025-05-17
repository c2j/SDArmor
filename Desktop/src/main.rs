use anyhow::Result;
use eframe::{App, CreationContext, Frame, NativeOptions};
use egui::{Context, CentralPanel, SidePanel, TopBottomPanel, Visuals};
use log::{info, warn, error};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

mod scanner;
mod ui;
mod rules;
mod report;
mod network;
mod config;
mod utils;
mod visualization;



const APP_NAME: &str = "SDChat Security Scanner";

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

        Self {
            scan_path: None,
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
        }
    }
}

impl App for ScannerApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {

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
                            }
                            self.scan_in_progress = false;
                        } else if progress < 0.0 {
                            // 扫描出错
                            error!("Scan encountered an error");
                            self.scan_in_progress = false;
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
            ui.heading("Navigation");
            if ui.button("Scan Configuration").clicked() {
                self.ui_state.active_tab = ui::ActiveTab::ScanConfig;
            }
            if ui.button("Dashboard").clicked() {
                self.ui_state.active_tab = ui::ActiveTab::Dashboard;
            }
            if ui.button("Vulnerability Details").clicked() {
                self.ui_state.active_tab = ui::ActiveTab::VulnerabilityDetails;
            }
            if ui.button("Settings").clicked() {
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

        // 底部状态栏
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.scan_in_progress {
                    ui.spinner();

                    // Basic progress percentage
                    ui.label(format!("Scanning... {}%", (self.scan_progress * 100.0) as i32));

                    // Add separator
                    ui.separator();

                    // Show detailed scan information if we have results
                    if let Some(results) = &self.scan_results {
                        // Files scanned
                        let total_files = results.stats.total_files;
                        let scanned_files = results.stats.scanned_files;
                        ui.label(format!("Files: {}/{}", scanned_files, total_files));

                        // Vulnerabilities found
                        let total_vulns = results.stats.critical + results.stats.high +
                                         results.stats.medium + results.stats.low;
                        ui.label(format!("Found: {} vulnerabilities", total_vulns));

                        // Breakdown by severity (with colors)
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

                        // Elapsed time and scan rate
                        let elapsed_ms = results.stats.scan_time_ms;
                        if elapsed_ms > 0 {
                            ui.separator();
                            ui.label(format!("Time: {:.1}s", elapsed_ms as f32 / 1000.0));

                            // Calculate and show scan rate (files per second)
                            if elapsed_ms > 1000 { // Only show rate after 1 second
                                let rate = (scanned_files as f32 * 1000.0) / elapsed_ms as f32;
                                ui.label(format!("Rate: {:.1} files/sec", rate));
                            }
                        }
                    }
                } else if self.scan_results.is_some() {
                    let results = self.scan_results.as_ref().unwrap();
                    let total_vulns = results.stats.critical + results.stats.high +
                                     results.stats.medium + results.stats.low;

                    ui.label(format!("Scan complete - Found {} vulnerabilities", total_vulns));

                    // Add breakdown by severity
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

                    // Show scan time
                    ui.separator();
                    ui.label(format!("Scan time: {:.1}s", results.stats.scan_time_ms as f32 / 1000.0));
                } else {
                    ui.label("Ready");
                }
            });
        });
    }
}

impl ScannerApp {
    fn new(_cc: &CreationContext) -> Self {
        let mut app = Self::default();

        // 初始化规则管理器
        let mut rule_manager = rules::RuleManager::new(&app.ui_state.server_url);

        // 尝试加载默认规则
        if let Err(e) = rule_manager.load_default_rules() {
            error!("Failed to load default rules: {}", e);
        } else {
            app.rules_loaded = true;
        }

        app.rule_manager = Some(rule_manager);
        info!("Application initialized with Tokio runtime and default rules");
        app
    }

    fn ui_scan_config(&mut self, ui: &mut egui::Ui) {
        ui.heading("Scan Configuration");

        ui.horizontal(|ui| {
            ui.label("Target Path:");
            if ui.button("Select Directory").clicked() {
                #[cfg(feature = "file_dialog")]
                {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.scan_path = Some(path);
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
            ui.label(format!("Selected: {}", path.display()));
        }

        ui.separator();
        ui.heading("Scanning Rules");

        // 显示规则信息
        if self.downloading_rules {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Downloading latest rules...");
            });
        } else if let Some(error) = &self.download_error {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::RED, format!("Rule download failed: {}", error));
            });
            if ui.button("Retry").clicked() {
                self.download_error = None;
                self.download_rules();
            }
        } else if self.rules_loaded {
            if let Some(rule_manager) = &self.rule_manager {
                let ruleset = rule_manager.ruleset();
                let stats = ruleset.stats();

                ui.label(format!("Rules Version: {}", ruleset.version));
                ui.label(format!("File Types: {}", stats.file_types));
                ui.label(format!("Total Patterns: {}", stats.patterns));

                ui.collapsing("Vulnerability Patterns by Severity", |ui| {
                    ui.label(format!("Critical: {}", stats.critical));
                    ui.label(format!("High: {}", stats.high));
                    ui.label(format!("Medium: {}", stats.medium));
                    ui.label(format!("Low: {}", stats.low));
                });

                // 显示文件类型和规则数量
                ui.collapsing("Supported File Types", |ui| {
                    for file_type in &ruleset.file_types {
                        ui.label(format!("{}: {} patterns", file_type.name, file_type.patterns.len()));
                    }
                });

                if ui.button("Update Rules").clicked() && !self.downloading_rules {
                    self.download_rules();
                }
            }
        } else {
            ui.label("No rules loaded. Please download rules first.");
            if ui.button("Download Rules").clicked() {
                self.download_rules();
            }
        }

        ui.separator();
        // 只有在有扫描规则和选择了路径的情况下才启用扫描按钮
        if self.scan_path.is_some() && self.rules_loaded {
            if ui.button("Start Scan").clicked() {
                self.start_scan();
            }
        } else {
            // 禁用状态的按钮
            let btn = egui::Button::new("Start Scan");
            ui.add_enabled(false, btn);
            if self.scan_path.is_none() {
                ui.label("Please select a directory to scan");
            }
            if !self.rules_loaded {
                ui.label("Please load scanning rules first");
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
            // 显示漏洞详情列表
            for (i, snippet) in results.code_snippets.iter().enumerate() {
                ui.collapsing(format!("{}: {} ({})", i+1, snippet.title, &snippet.severity.to_string()), |ui| {
                    ui.label(format!("File: {}", snippet.file_path.display()));
                    ui.label(format!("Line: {}", snippet.line_number));
                    ui.code(snippet.code.clone());
                    ui.label(format!("Description: {}", snippet.description));
                });
            }
        } else {
            ui.label("No vulnerabilities to display. Run a scan first.");
        }
    }

    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");

        // 主题设置
        ui.checkbox(&mut self.ui_state.dark_mode, "Dark Mode");
        if self.ui_state.dark_mode_changed {
            if self.ui_state.dark_mode {
                ui.ctx().set_visuals(Visuals::dark());
            } else {
                ui.ctx().set_visuals(Visuals::light());
            }
            self.ui_state.dark_mode_changed = false;
        }

        // 服务器设置
        ui.horizontal(|ui| {
            ui.label("Server URL:");
            ui.text_edit_singleline(&mut self.ui_state.server_url);
        });
    }

    // 下载最新规则
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
        let config = scanner::ScanConfig {
            target_path: path,
            rules,
            ..Default::default()
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
}

fn main() -> Result<()> {
    // 初始化日志
    env_logger::init();

    // Get version from package info
    let version = env!("CARGO_PKG_VERSION");

    info!("Starting {} v{}", APP_NAME, version);

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
        APP_NAME,
        options,
        Box::new(|cc| Box::new(ScannerApp::new(cc)))
    ).map_err(|e| anyhow::anyhow!("Application error: {}", e))?;

    Ok(())
}