use egui::{Context, Ui, Color32, RichText, Stroke};
use std::path::PathBuf;

/// Represents the active tab in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    ScanConfig,
    Dashboard,
    VulnerabilityDetails,
    Settings,
}

impl Default for ActiveTab {
    fn default() -> Self {
        Self::ScanConfig
    }
}

/// 上传对话框数据
#[derive(Debug, Default)]
pub struct UploadDialogData {
    pub entry_index: usize,
    pub report_path: Option<PathBuf>,
    pub application_name: String,
    pub uploader_name: String,
    pub rule_version: String,
    pub notes: String,
}

/// Stores the UI state of the application
#[derive(Debug)]
pub struct UiState {
    pub active_tab: ActiveTab,
    pub dark_mode: bool,
    pub dark_mode_changed: bool,
    pub server_url: String,
    pub show_advanced_options: bool,
    pub scan_result_filter: ScanResultFilter,
    // 上传对话框相关
    pub show_upload_dialog: bool,
    pub upload_dialog_data: UploadDialogData,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            active_tab: ActiveTab::default(),
            dark_mode: true,
            dark_mode_changed: false,
            server_url: "https://rules.sdchat-scanner.com".to_string(),
            show_advanced_options: false,
            scan_result_filter: ScanResultFilter::default(),
            show_upload_dialog: false,
            upload_dialog_data: UploadDialogData::default(),
        }
    }
}

/// Filter options for scan results
#[derive(Debug, Default)]
pub struct ScanResultFilter {
    pub show_critical: bool,
    pub show_high: bool,
    pub show_medium: bool,
    pub show_low: bool,
    pub file_filter: Option<String>,
}

/// UI helper functions
pub mod helpers {
    use super::*;

    pub fn severity_color(severity: &str) -> Color32 {
        match severity.to_lowercase().as_str() {
            "critical" => Color32::from_rgb(255, 0, 0),
            "high" => Color32::from_rgb(255, 120, 0),
            "medium" => Color32::from_rgb(255, 204, 0),
            "low" => Color32::from_rgb(0, 128, 255),
            _ => Color32::GRAY,
        }
    }

    pub fn severity_text(ui: &mut Ui, severity: &str) -> RichText {
        RichText::new(severity).color(severity_color(severity)).strong()
    }

    pub fn progress_bar(ui: &mut Ui, progress: f32, text: Option<&str>) {
        let rect = ui.available_rect_before_wrap();
        let width = rect.width();
        let height = 20.0;

        let bar_rect = egui::Rect::from_min_size(
            rect.min,
            egui::vec2(width * progress.clamp(0.0, 1.0), height),
        );

        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(width, height)),
            3.0,
            Color32::from_gray(40),
        );

        ui.painter().rect_filled(
            bar_rect,
            3.0,
            Color32::from_rgb(50, 150, 255),
        );

        if let Some(text) = text {
            let text_pos = rect.min + egui::vec2(width / 2.0, height / 2.0);
            ui.painter().text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::default(),
                Color32::WHITE,
            );
        }

        ui.add_space(height + 4.0);
    }

    pub fn file_path_view(ui: &mut Ui, path: &PathBuf) {
        let path_str = path.to_string_lossy();
        let parts: Vec<&str> = path_str.split('/').collect();

        ui.horizontal(|ui| {
            for (i, part) in parts.iter().enumerate() {
                if i > 0 {
                    ui.label("/");
                }
                ui.label(*part);
            }
        });
    }
}