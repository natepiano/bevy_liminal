use bevy::prelude::*;

// auto-mode
pub(super) const AUTO_EXIT_DELAY_SECS: f32 = 2.0;
pub(super) const AUTO_MODE_ENV_VAR: &str = "BENCHMARK_AUTO";
pub(super) const AUTO_STARTUP_DELAY_SECS: f32 = 5.0;

// camera
pub(super) const CAMERA_LOOK_AT: Vec3 = Vec3::new(0.0, 4.0, 0.0);
pub(super) const CAMERA_POSITION: Vec3 = Vec3::new(8.0, 2.0, 14.0);

// cube fill ratios
pub(super) const CUBE_FILL_RATIO_00005: f32 = 0.45;
pub(super) const CUBE_FILL_RATIO_00010: f32 = 0.65;
pub(super) const CUBE_FILL_RATIO_00100: f32 = 0.55;
pub(super) const CUBE_FILL_RATIO_01000: f32 = 0.35;
pub(super) const CUBE_FILL_RATIO_10000: f32 = 0.25;
pub(super) const CUBE_FILL_RATIO_50000: f32 = 0.15;

// grid layout
pub(super) const DEPTH_SPACING_MULTIPLIER: f32 = 3.0;
pub(super) const GRID_3D_COLUMNS: u32 = 10;
pub(super) const GRID_3D_ROWS: u32 = 10;
pub(super) const GRID_CENTER_DIVISOR: f32 = 2.0;
pub(super) const GRID_CENTER_OFFSET: f32 = 1.0;
pub(super) const GRID_FILL_FRACTION: f32 = 0.95;
pub(super) const GRID_TO_3D_THRESHOLD: u32 = 100;
pub(super) const GROUND_PLANE_SIZE: f32 = 100.0;
pub(super) const GROUND_PLANE_SUBDIVISIONS: u32 = 10;
pub(super) const GROUND_PLANE_Y: f32 = -3.0;
pub(super) const VIEWPORT_FOV_DIVISOR: f32 = 2.0;
pub(super) const VIEWPORT_HEIGHT_MULTIPLIER: f32 = 2.0;

// HUD
pub(super) const HEADS_UP_DISPLAY_FONT_SIZE: f32 = 18.0;
pub(super) const HEADS_UP_DISPLAY_PADDING: f32 = 10.0;
pub(super) const HEADS_UP_DISPLAY_UPDATE_INTERVAL: f32 = 0.25;

// lighting
pub(super) const AMBIENT_LIGHT_BRIGHTNESS: f32 = 200.0;
pub(super) const LIGHT_INTENSITY: f32 = 10_000_000.0;
pub(super) const LIGHT_POSITION: Vec3 = Vec3::new(8.0, 16.0, 8.0);
pub(super) const LIGHT_RANGE: f32 = 100.0;

// measurement
pub(super) const MEASURE_FRAMES: u32 = 600;
pub(super) const MILLISECONDS_PER_SECOND: f64 = 1000.0;
pub(super) const WARMUP_FRAMES: u32 = 120;

// outline defaults
pub(super) const DEFAULT_OUTLINE_INTENSITY: f32 = 1.0;
pub(super) const DEFAULT_OUTLINE_WIDTH: f32 = 5.0;
