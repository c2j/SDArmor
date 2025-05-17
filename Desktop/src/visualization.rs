use anyhow::Result;
use egui::{Color32, Rect, Stroke, Ui, Vec2};
use std::f32::consts::PI;
use std::path::PathBuf;

use crate::rules::Severity;
use crate::scanner::Hotspot;

/// 3D visualization camera
#[derive(Debug, Clone)]
pub struct Camera3D {
    pub rotation_x: f32,
    pub rotation_y: f32,
    pub distance: f32,
    pub target: [f32; 3],
    pub fov: f32,
    pub aspect_ratio: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            rotation_x: 0.5,
            rotation_y: 0.5,
            distance: 300.0,
            target: [0.0, 0.0, 0.0],
            fov: 45.0,
            aspect_ratio: 1.5,
        }
    }
}

/// 3D point with depth information for rendering
#[derive(Debug, Clone)]
struct Point3D {
    world_pos: [f32; 3],
    screen_pos: [f32; 2],
    depth: f32,
    color: Color32,
    size: f32,
    severity: Severity,
    file_path: PathBuf,
    rule_id: String,
}

/// 3D Heatmap visualization
pub struct Heatmap3D {
    points: Vec<Point3D>,
    camera: Camera3D,
    selected_point: Option<usize>,
    rotate_speed: f32,
    auto_rotate: bool,
    rotation_angle: f32,
    mouse_drag: Option<egui::Pos2>,
}

impl Default for Heatmap3D {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            camera: Camera3D::default(),
            selected_point: None,
            rotate_speed: 0.01,
            auto_rotate: true,
            rotation_angle: 0.0,
            mouse_drag: None,
        }
    }
}

impl Heatmap3D {
    /// Create a new empty 3D heatmap visualization
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the visualization with new hotspots
    pub fn update_hotspots(&mut self, hotspots: &[Hotspot]) {
        self.points.clear();

        for hotspot in hotspots {
            let color = match hotspot.severity {
                Severity::Critical => Color32::from_rgb(255, 0, 0),
                Severity::High => Color32::from_rgb(255, 120, 0),
                Severity::Medium => Color32::from_rgb(255, 204, 0),
                Severity::Low => Color32::from_rgb(0, 128, 255),
            };

            let size = match hotspot.severity {
                Severity::Critical => 8.0,
                Severity::High => 6.0,
                Severity::Medium => 5.0,
                Severity::Low => 4.0,
            };

            self.points.push(Point3D {
                world_pos: [hotspot.x, hotspot.y, hotspot.z],
                screen_pos: [0.0, 0.0], // Will be calculated during rendering
                depth: 0.0,             // Will be calculated during rendering
                color,
                size,
                severity: hotspot.severity.clone(),
                file_path: hotspot.file_path.clone(),
                rule_id: hotspot.rule_id.clone(),
            });
        }
    }

    /// Set auto-rotation state
    pub fn set_auto_rotate(&mut self, auto_rotate: bool) {
        self.auto_rotate = auto_rotate;
    }

    /// Render the 3D heatmap visualization
    pub fn render(&mut self, ui: &mut Ui) -> Option<(PathBuf, String)> {
        let mut clicked_hotspot = None;

        // Get the available rect for the visualization
        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

        // Update camera aspect ratio based on rect
        self.camera.aspect_ratio = rect.width() / rect.height();

        // Handle mouse interactions
        if response.dragged() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                if let Some(prev_pos) = self.mouse_drag {
                    let delta = pointer_pos - prev_pos;
                    self.camera.rotation_y += delta.x * 0.01;
                    self.camera.rotation_x += delta.y * 0.01;
                }
                self.mouse_drag = Some(pointer_pos);
            }
        } else {
            self.mouse_drag = None;

            // Auto-rotation when not interacting
            if self.auto_rotate {
                self.rotation_angle += self.rotate_speed;
                self.camera.rotation_y = self.rotation_angle;
            }
        }

        // Handle zoom with scroll
        if response.hovered() {
            ui.input(|i| {
                let scroll_delta = i.scroll_delta.y;
                if scroll_delta != 0.0 {
                    self.camera.distance -= scroll_delta * 0.5;
                    self.camera.distance = self.camera.distance.clamp(50.0, 500.0);
                }
            });
        }

        if response.clicked() {
            // Find the closest point to the click
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let mut closest_dist = f32::MAX;
                let mut closest_idx = None;

                for (idx, point) in self.points.iter().enumerate() {
                    let dx = pointer_pos.x - point.screen_pos[0];
                    let dy = pointer_pos.y - point.screen_pos[1];
                    let dist_sq = dx * dx + dy * dy;

                    // Check if click is within the point's rendered size
                    if dist_sq < (point.size * 1.5).powi(2) && point.depth < closest_dist {
                        closest_dist = point.depth;
                        closest_idx = Some(idx);
                    }
                }

                self.selected_point = closest_idx;

                // Return the clicked hotspot information
                if let Some(idx) = closest_idx {
                    let point = &self.points[idx];
                    clicked_hotspot = Some((point.file_path.clone(), point.rule_id.clone()));
                }
            }
        }

        // Project 3D points to 2D
        self.project_points(rect);

        // Sort points by depth for correct rendering
        self.points.sort_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap_or(std::cmp::Ordering::Equal));

        // Draw coordinate system
        self.draw_axes(ui, rect);

        // Draw all points
        for (idx, point) in self.points.iter().enumerate() {
            let pos = egui::pos2(point.screen_pos[0], point.screen_pos[1]);
            let size = point.size * (1.0 - 0.5 * point.depth / 500.0).max(0.2);

            // Highlight selected point
            let color = if Some(idx) == self.selected_point {
                Color32::WHITE
            } else {
                point.color
            };

            let rect = Rect::from_center_size(pos, Vec2::splat(size * 2.0));
            ui.painter().circle_filled(pos, size, color);

            // Draw outline for selected point
            if Some(idx) == self.selected_point {
                ui.painter().circle_stroke(pos, size + 2.0, Stroke::new(1.0, Color32::WHITE));
            }
        }

        // Draw legend
        self.draw_legend(ui, rect);

        clicked_hotspot
    }

    /// Draw coordinate system axes
    fn draw_axes(&self, ui: &mut Ui, rect: Rect) {
        let center = rect.center();
        let axis_length = 40.0;

        // Calculate axis endpoints
        let rot_x = self.camera.rotation_x;
        let rot_y = self.camera.rotation_y;

        let x_dir = [
            rot_y.cos(),
            0.0,
            rot_y.sin()
        ];

        let y_dir = [
            rot_x.sin() * rot_y.sin(),
            rot_x.cos(),
            -rot_x.sin() * rot_y.cos()
        ];

        let z_dir = [
            -rot_x.cos() * rot_y.sin(),
            rot_x.sin(),
            rot_x.cos() * rot_y.cos()
        ];

        let x_end = egui::pos2(
            center.x + x_dir[0] * axis_length,
            center.y - x_dir[1] * axis_length
        );

        let y_end = egui::pos2(
            center.x + y_dir[0] * axis_length,
            center.y - y_dir[1] * axis_length
        );

        let z_end = egui::pos2(
            center.x + z_dir[0] * axis_length,
            center.y - z_dir[1] * axis_length
        );

        // Draw axes
        ui.painter().line_segment([center, x_end], Stroke::new(1.0, Color32::RED));
        ui.painter().line_segment([center, y_end], Stroke::new(1.0, Color32::GREEN));
        ui.painter().line_segment([center, z_end], Stroke::new(1.0, Color32::BLUE));

        // Draw axis labels
        ui.painter().text(x_end, egui::Align2::CENTER_CENTER, "X", egui::FontId::default(), Color32::RED);
        ui.painter().text(y_end, egui::Align2::CENTER_CENTER, "Y", egui::FontId::default(), Color32::GREEN);
        ui.painter().text(z_end, egui::Align2::CENTER_CENTER, "Z", egui::FontId::default(), Color32::BLUE);
    }

    /// Draw severity legend
    fn draw_legend(&self, ui: &mut Ui, rect: Rect) {
        let text_style = egui::TextStyle::Small;
        let legend_width = 100.0;
        let legend_height = 80.0;
        let padding = 8.0;

        let legend_rect = Rect::from_min_size(
            egui::pos2(rect.max.x - legend_width - padding, rect.min.y + padding),
            egui::vec2(legend_width, legend_height),
        );

        ui.painter().rect_filled(
            legend_rect,
            3.0,
            Color32::from_rgba_premultiplied(0, 0, 0, 180),
        );

        let mut y_offset = legend_rect.min.y + 12.0;
        let x_pos = legend_rect.min.x + 30.0;
        let circle_pos = legend_rect.min.x + 15.0;

        // Critical
        ui.painter().circle_filled(
            egui::pos2(circle_pos, y_offset),
            6.0,
            Color32::from_rgb(255, 0, 0),
        );
        ui.painter().text(
            egui::pos2(x_pos, y_offset),
            egui::Align2::LEFT_CENTER,
            "Critical",
            egui::FontId::new(11.0, egui::FontFamily::Proportional),
            Color32::WHITE,
        );
        y_offset += 15.0;

        // High
        ui.painter().circle_filled(
            egui::pos2(circle_pos, y_offset),
            5.0,
            Color32::from_rgb(255, 120, 0),
        );
        ui.painter().text(
            egui::pos2(x_pos, y_offset),
            egui::Align2::LEFT_CENTER,
            "High",
            egui::FontId::new(11.0, egui::FontFamily::Proportional),
            Color32::WHITE,
        );
        y_offset += 15.0;

        // Medium
        ui.painter().circle_filled(
            egui::pos2(circle_pos, y_offset),
            4.0,
            Color32::from_rgb(255, 204, 0),
        );
        ui.painter().text(
            egui::pos2(x_pos, y_offset),
            egui::Align2::LEFT_CENTER,
            "Medium",
            egui::FontId::new(11.0, egui::FontFamily::Proportional),
            Color32::WHITE,
        );
        y_offset += 15.0;

        // Low
        ui.painter().circle_filled(
            egui::pos2(circle_pos, y_offset),
            3.0,
            Color32::from_rgb(0, 128, 255),
        );
        ui.painter().text(
            egui::pos2(x_pos, y_offset),
            egui::Align2::LEFT_CENTER,
            "Low",
            egui::FontId::new(11.0, egui::FontFamily::Proportional),
            Color32::WHITE,
        );
    }

    /// Project 3D points to 2D screen coordinates
    fn project_points(&mut self, rect: Rect) {
        let center = rect.center();
        let rot_x = self.camera.rotation_x;
        let rot_y = self.camera.rotation_y;

        for point in &mut self.points {
            // Translate point relative to camera target
            let rel_pos = [
                point.world_pos[0] - self.camera.target[0],
                point.world_pos[1] - self.camera.target[1],
                point.world_pos[2] - self.camera.target[2],
            ];

            // Rotate around Y axis
            let rotated_y = [
                rel_pos[0] * rot_y.cos() + rel_pos[2] * rot_y.sin(),
                rel_pos[1],
                -rel_pos[0] * rot_y.sin() + rel_pos[2] * rot_y.cos(),
            ];

            // Rotate around X axis
            let rotated = [
                rotated_y[0],
                rotated_y[1] * rot_x.cos() - rotated_y[2] * rot_x.sin(),
                rotated_y[1] * rot_x.sin() + rotated_y[2] * rot_x.cos(),
            ];

            // Move point based on camera distance
            let z = rotated[2] + self.camera.distance;

            // Calculate perspective
            let fov_rad = self.camera.fov * PI / 180.0;
            let scale = 1.0 / (z * fov_rad.tan());

            // Project to screen coordinates
            point.screen_pos[0] = center.x + rotated[0] * scale * rect.height();
            point.screen_pos[1] = center.y - rotated[1] * scale * rect.height();
            point.depth = z;
        }
    }
}

/// A circular progress indicator
pub struct CircularProgress {
    value: f32,
    radius: f32,
    thickness: f32,
    bg_color: Color32,
    fg_color: Color32,
    label: Option<String>,
}

impl Default for CircularProgress {
    fn default() -> Self {
        Self {
            value: 0.0,
            radius: 50.0,
            thickness: 8.0,
            bg_color: Color32::from_gray(30),
            fg_color: Color32::from_rgb(50, 150, 255),
            label: None,
        }
    }
}

impl CircularProgress {
    /// Create a new circular progress indicator
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Set the radius of the circular progress indicator
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Set the thickness of the circular progress indicator
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }

    /// Set the background color of the circular progress indicator
    pub fn bg_color(mut self, color: Color32) -> Self {
        self.bg_color = color;
        self
    }

    /// Set the foreground color of the circular progress indicator
    pub fn fg_color(mut self, color: Color32) -> Self {
        self.fg_color = color;
        self
    }

    /// Set the label to show in the center of the circular progress indicator
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Update the progress value
    pub fn set_value(&mut self, value: f32) {
        self.value = value.clamp(0.0, 1.0);
    }

    /// Render the circular progress indicator
    pub fn render(&self, ui: &mut Ui) -> egui::Response {
        let desired_size = Vec2::splat(self.radius * 2.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let center = rect.center();
            let painter = ui.painter();

            // Draw background circle
            painter.circle_stroke(
                center,
                self.radius,
                Stroke::new(self.thickness, self.bg_color),
            );

            // Draw progress arc
            if self.value > 0.0 {
                let angle = 2.0 * PI * self.value;
                let points = (angle / 0.05).ceil() as usize;
                let points = points.max(2);

                let mut points_vec = Vec::with_capacity(points);
                points_vec.push(center);

                for i in 0..=points {
                    let a = -PI / 2.0 + (i as f32) * angle / (points as f32);
                    let x = center.x + self.radius * a.cos();
                    let y = center.y + self.radius * a.sin();
                    points_vec.push(egui::pos2(x, y));
                }

                painter.add(egui::Shape::Path(egui::epaint::PathShape {
                    points: points_vec,
                    closed: false,
                    fill: Color32::TRANSPARENT,
                    stroke: Stroke::new(self.thickness, self.fg_color),
                }));
            }

            // Draw label text
            if let Some(label) = &self.label {
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(self.radius * 0.5),
                    Color32::WHITE,
                );
            } else {
                // Draw percentage if no label
                let percentage = format!("{}%", (self.value * 100.0) as i32);
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    percentage,
                    egui::FontId::proportional(self.radius * 0.5),
                    Color32::WHITE,
                );
            }
        }

        response
    }
}