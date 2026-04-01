//! Performance benchmark spawning many outlined meshes with FPS tracking.
use std::env;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write as IoWrite;

use bevy::color::palettes::css::DARK_SEA_GREEN;
use bevy::color::palettes::css::YELLOW;
use bevy::diagnostic::DiagnosticsStore;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::ecs::system::SystemParam;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy::winit::WinitSettings;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_brp_extras::PortDisplay;
use bevy_kana::ToF32;
use bevy_kana::ToF64;
use bevy_kana::ToU32;
use bevy_kana::ToUsize;
use bevy_liminal::LiminalPlugin;
use bevy_liminal::Outline;
use bevy_liminal::OutlineCamera;
use bevy_liminal::OutlineMethod;
use bevy_liminal::OverlapMode;
use bevy_window_manager::WindowManagerPlugin;
use rand::RngExt;

// --- Main ---

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_liminal benchmark".into(),
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(BrpExtrasPlugin::default().port_in_title(PortDisplay::NonDefault))
        .add_plugins(LiminalPlugin)
        .add_plugins(WindowManagerPlugin)
        .insert_resource(WinitSettings::continuous())
        .insert_resource(BenchmarkState::new())
        .insert_resource(HudUpdateTimer(Timer::from_seconds(
            HUD_UPDATE_INTERVAL,
            TimerMode::Repeating,
        )))
        .add_systems(Startup, setup_benchmark)
        .add_systems(
            Update,
            (
                benchmark_tick,
                handle_input.run_if(on_message::<KeyboardInput>),
                update_hud,
            ),
        )
        .run();
}

// --- Constants ---

const AMBIENT_LIGHT_BRIGHTNESS: f32 = 200.0;
const AUTO_EXIT_DELAY_SECS: f32 = 2.0;
const AUTO_MODE_ENV_VAR: &str = "BENCHMARK_AUTO";
const AUTO_STARTUP_DELAY_SECS: f32 = 5.0;
const CAMERA_LOOK_AT: Vec3 = Vec3::new(0.0, 4.0, 0.0);
const CAMERA_POSITION: Vec3 = Vec3::new(8.0, 2.0, 14.0);
const CUBE_FILL_RATIO_5: f32 = 0.45;
const CUBE_FILL_RATIO_10: f32 = 0.65;
const CUBE_FILL_RATIO_100: f32 = 0.55;
const CUBE_FILL_RATIO_1000: f32 = 0.35;
const CUBE_FILL_RATIO_10000: f32 = 0.25;
const CUBE_FILL_RATIO_50000: f32 = 0.15;
const DEFAULT_OUTLINE_INTENSITY: f32 = 1.0;
const DEFAULT_OUTLINE_WIDTH: f32 = 5.0;
const DEPTH_SPACING_MULTIPLIER: f32 = 3.0;
const GRID_FILL_FRACTION: f32 = 0.95;
const GROUND_PLANE_SIZE: f32 = 100.0;
const GROUND_PLANE_Y: f32 = -3.0;
const HUD_FONT_SIZE: f32 = 18.0;
const HUD_PADDING: f32 = 10.0;
const HUD_UPDATE_INTERVAL: f32 = 0.25;
const LIGHT_INTENSITY: f32 = 10_000_000.0;
const LIGHT_POSITION: Vec3 = Vec3::new(8.0, 16.0, 8.0);
const LIGHT_RANGE: f32 = 100.0;
const MEASURE_FRAMES: u32 = 600;
const MS_PER_SECOND: f64 = 1000.0;
const WARMUP_FRAMES: u32 = 120;

// --- Scenario Definitions ---

#[derive(Clone, Copy)]
struct ScenarioDefinition {
    name: &'static str,
    key:  KeyCode,
    kind: ScenarioKind,
}

#[derive(Clone, Copy)]
enum ScenarioKind {
    Grid {
        count:     u32,
        width:     f32,
        cube_fill: f32,
    },
}

const SCENARIOS: &[ScenarioDefinition] = &[
    ScenarioDefinition {
        name: "Entities1",
        key:  KeyCode::Digit1,
        kind: ScenarioKind::Grid {
            count:     1,
            width:     DEFAULT_OUTLINE_WIDTH,
            cube_fill: CUBE_FILL_RATIO_5,
        },
    },
    ScenarioDefinition {
        name: "Entities5",
        key:  KeyCode::Digit2,
        kind: ScenarioKind::Grid {
            count:     5,
            width:     DEFAULT_OUTLINE_WIDTH,
            cube_fill: CUBE_FILL_RATIO_5,
        },
    },
    ScenarioDefinition {
        name: "Entities10",
        key:  KeyCode::Digit3,
        kind: ScenarioKind::Grid {
            count:     10,
            width:     DEFAULT_OUTLINE_WIDTH,
            cube_fill: CUBE_FILL_RATIO_10,
        },
    },
    ScenarioDefinition {
        name: "Entities100",
        key:  KeyCode::Digit4,
        kind: ScenarioKind::Grid {
            count:     100,
            width:     DEFAULT_OUTLINE_WIDTH,
            cube_fill: CUBE_FILL_RATIO_100,
        },
    },
    ScenarioDefinition {
        name: "Entities1000",
        key:  KeyCode::Digit5,
        kind: ScenarioKind::Grid {
            count:     1000,
            width:     DEFAULT_OUTLINE_WIDTH,
            cube_fill: CUBE_FILL_RATIO_1000,
        },
    },
    ScenarioDefinition {
        name: "Entities10000",
        key:  KeyCode::Digit6,
        kind: ScenarioKind::Grid {
            count:     10000,
            width:     DEFAULT_OUTLINE_WIDTH,
            cube_fill: CUBE_FILL_RATIO_10000,
        },
    },
    ScenarioDefinition {
        name: "Entities50000",
        key:  KeyCode::Digit7,
        kind: ScenarioKind::Grid {
            count:     50000,
            width:     DEFAULT_OUTLINE_WIDTH,
            cube_fill: CUBE_FILL_RATIO_50000,
        },
    },
];

// --- Benchmark State ---

#[derive(PartialEq, Eq)]
enum BenchmarkMode {
    Auto,
    Interactive,
}

enum BenchmarkPhase {
    Idle,
    StartupDelay,
    Setup,
    Warmup,
    Measure,
    Analyze,
    ExitDelay,
}

struct ScenarioResult {
    name:      String,
    frames:    u32,
    avg_ms:    f64,
    median_ms: f64,
    p95_ms:    f64,
    p99_ms:    f64,
    min_ms:    f64,
    max_ms:    f64,
}

impl ScenarioResult {
    fn avg_fps(&self) -> f64 {
        if self.avg_ms > 0.0 {
            MS_PER_SECOND / self.avg_ms
        } else {
            0.0
        }
    }
}

#[derive(Resource)]
struct BenchmarkState {
    mode:             BenchmarkMode,
    current_scenario: usize,
    outline_enabled:  bool,
    outline_mode:     OutlineMethod,
    phase:            BenchmarkPhase,
    frame_counter:    u32,
    frame_times:      Vec<f64>,
    results:          Vec<ScenarioResult>,
    startup_timer:    Timer,
    exit_timer:       Timer,
    exit_on_complete: bool,
}

impl BenchmarkState {
    fn new() -> Self {
        let exit_on_complete = env::var(AUTO_MODE_ENV_VAR).is_ok_and(|v| v == "1");
        let (mode, phase) = if exit_on_complete {
            (BenchmarkMode::Auto, BenchmarkPhase::StartupDelay)
        } else {
            (BenchmarkMode::Interactive, BenchmarkPhase::Idle)
        };

        Self {
            mode,
            current_scenario: 0,
            outline_enabled: false,
            outline_mode: OutlineMethod::default(),
            phase,
            frame_counter: 0,
            frame_times: Vec::with_capacity(MEASURE_FRAMES.to_usize()),
            results: Vec::with_capacity(SCENARIOS.len() * 2),
            startup_timer: Timer::from_seconds(AUTO_STARTUP_DELAY_SECS, TimerMode::Once),
            exit_timer: Timer::from_seconds(AUTO_EXIT_DELAY_SECS, TimerMode::Once),
            exit_on_complete,
        }
    }

    fn result_name(&self) -> String {
        let scenario = &SCENARIOS[self.current_scenario];
        let suffix = if self.outline_enabled { "on" } else { "off" };
        let mode_label = outline_mode_label(self.outline_mode);
        format!("{} {suffix} ({mode_label})", scenario.name)
    }
}

const fn outline_mode_label(mode: OutlineMethod) -> &'static str {
    match mode {
        OutlineMethod::JumpFlood => "JumpFlood",
        OutlineMethod::WorldHull => "WorldHull",
        OutlineMethod::ScreenHull => "ScreenHull",
        _ => unreachable!(),
    }
}

const fn next_outline_mode(mode: OutlineMethod) -> OutlineMethod {
    match mode {
        OutlineMethod::JumpFlood => OutlineMethod::WorldHull,
        OutlineMethod::WorldHull => OutlineMethod::ScreenHull,
        OutlineMethod::ScreenHull => OutlineMethod::JumpFlood,
        _ => unreachable!(),
    }
}

// --- Marker Components ---

#[derive(Component)]
struct BenchmarkEntity;

#[derive(Component)]
struct HudText;

#[derive(Resource)]
struct HudUpdateTimer(Timer);

// --- Setup ---

fn setup_benchmark(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(CAMERA_POSITION).looking_at(CAMERA_LOOK_AT, Vec3::Y),
        OutlineCamera,
        AmbientLight {
            brightness: AMBIENT_LIGHT_BRIGHTNESS,
            ..default()
        },
    ));

    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: LIGHT_INTENSITY,
            range: LIGHT_RANGE,
            ..default()
        },
        Transform::from_translation(LIGHT_POSITION),
    ));

    commands.spawn((
        Mesh3d(
            meshes.add(
                Plane3d::default()
                    .mesh()
                    .size(GROUND_PLANE_SIZE, GROUND_PLANE_SIZE)
                    .subdivisions(10),
            ),
        ),
        MeshMaterial3d(materials.add(Color::from(DARK_SEA_GREEN))),
        Transform::from_xyz(0.0, GROUND_PLANE_Y, 0.0),
    ));

    // HUD text
    commands.spawn((
        Text::new("Initializing benchmark..."),
        TextFont {
            font_size: HUD_FONT_SIZE,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(HUD_PADDING),
            left: Val::Px(HUD_PADDING),
            ..default()
        },
        HudText,
    ));
}

// --- Viewport Info ---

struct ViewportInfo {
    right:         Vec3,
    up:            Vec3,
    forward:       Vec3,
    center:        Vec3,
    usable_width:  f32,
    usable_height: f32,
}

fn compute_viewport_info(
    camera_transform: &Transform,
    projection: &Projection,
    window: &Window,
) -> ViewportInfo {
    let fov = match projection {
        Projection::Perspective(persp) => persp.fov,
        Projection::Orthographic(_) | Projection::Custom(_) => std::f32::consts::FRAC_PI_4,
    };

    let distance = camera_transform.translation.distance(CAMERA_LOOK_AT);
    let aspect = window.width() / window.height();
    let visible_height = 2.0 * distance * (fov / 2.0).tan();
    let visible_width = visible_height * aspect;

    let right = camera_transform.right().as_vec3();
    let up = camera_transform.up().as_vec3();
    let forward = camera_transform.forward().as_vec3();

    ViewportInfo {
        right,
        up,
        forward,
        center: CAMERA_LOOK_AT,
        usable_width: visible_width * GRID_FILL_FRACTION,
        usable_height: visible_height * GRID_FILL_FRACTION,
    }
}

// --- Scenario Spawning ---

fn spawn_scenario(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    scenario: &ScenarioDefinition,
    viewport: &ViewportInfo,
    outline_enabled: bool,
    outline_mode: OutlineMethod,
) {
    let ScenarioKind::Grid {
        count,
        width,
        cube_fill,
    } = scenario.kind;
    spawn_grid(
        commands,
        meshes,
        materials,
        GridSpawnSpec {
            count,
            width,
            cube_fill,
            viewport,
            outline_enabled,
            outline_mode,
        },
    );
}

fn random_outline_color() -> Color {
    let mut rng = rand::rng();
    Color::srgb(rng.random(), rng.random(), rng.random())
}

fn build_outline(width: f32, outline_mode: OutlineMethod) -> Outline {
    match outline_mode {
        OutlineMethod::JumpFlood => Outline::jump_flood(width)
            .with_intensity(DEFAULT_OUTLINE_INTENSITY)
            .with_color(random_outline_color())
            .build(),
        OutlineMethod::WorldHull => Outline::world_hull(width)
            .with_intensity(DEFAULT_OUTLINE_INTENSITY)
            .with_color(random_outline_color())
            .with_overlap(OverlapMode::PerMesh)
            .build(),
        OutlineMethod::ScreenHull => Outline::screen_hull(width)
            .with_intensity(DEFAULT_OUTLINE_INTENSITY)
            .with_color(random_outline_color())
            .with_overlap(OverlapMode::PerMesh)
            .build(),
        _ => unreachable!(),
    }
}

fn spawn_grid(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    spec: GridSpawnSpec<'_>,
) {
    let GridSpawnSpec {
        count,
        width,
        cube_fill,
        viewport,
        outline_enabled,
        outline_mode,
    } = spec;
    let mesh_handle = meshes.add(Cuboid::default());
    let material_handle = materials.add(Color::from(YELLOW));

    if count > 100 {
        spawn_3d_grid(
            commands,
            &mesh_handle,
            &material_handle,
            count,
            width,
            cube_fill,
            viewport,
            outline_enabled,
            outline_mode,
        );
    } else {
        // 2D grid
        let cols = count.to_f32().sqrt().ceil().to_u32();
        let rows = count.div_ceil(cols);
        let h_spacing = viewport.usable_width / cols.to_f32();
        let v_spacing = viewport.usable_height / rows.to_f32();
        let cube_scale = v_spacing * cube_fill;

        let mut spawned = 0u32;
        for row in 0..rows {
            for col in 0..cols {
                if spawned >= count {
                    break;
                }
                let col_offset = col.to_f32() - (cols.to_f32() - 1.0) / 2.0;
                let row_offset = row.to_f32() - (rows.to_f32() - 1.0) / 2.0;
                let position = viewport.center
                    + col_offset * h_spacing * viewport.right
                    + row_offset * v_spacing * viewport.up;
                let mut entity = commands.spawn((
                    Mesh3d(mesh_handle.clone()),
                    MeshMaterial3d(material_handle.clone()),
                    Transform::from_translation(position).with_scale(Vec3::splat(cube_scale)),
                    BenchmarkEntity,
                ));
                if outline_enabled {
                    entity.insert(build_outline(width, outline_mode));
                }
                spawned += 1;
            }
        }
    }
}

fn spawn_3d_grid(
    commands: &mut Commands,
    mesh_handle: &Handle<Mesh>,
    material_handle: &Handle<StandardMaterial>,
    count: u32,
    width: f32,
    cube_fill: f32,
    viewport: &ViewportInfo,
    outline_enabled: bool,
    outline_mode: OutlineMethod,
) {
    // 3D grid: 10x10 front face, depth layers as needed
    let cols: u32 = 10;
    let rows: u32 = 10;
    let face_size = cols * rows;
    let layers = count.div_ceil(face_size);
    let h_spacing = viewport.usable_width / cols.to_f32();
    let v_spacing = viewport.usable_height / rows.to_f32();
    let cube_scale = v_spacing * cube_fill;

    let mut spawned = 0u32;
    for depth in 0..layers {
        for row in 0..rows {
            for col in 0..cols {
                if spawned >= count {
                    break;
                }
                let col_offset = col.to_f32() - (cols.to_f32() - 1.0) / 2.0;
                let row_offset = row.to_f32() - (rows.to_f32() - 1.0) / 2.0;
                let depth_offset = depth.to_f32();
                let position = viewport.center
                    + col_offset * h_spacing * viewport.right
                    + row_offset * v_spacing * viewport.up
                    + depth_offset * v_spacing * DEPTH_SPACING_MULTIPLIER * viewport.forward;
                let mut entity = commands.spawn((
                    Mesh3d(mesh_handle.clone()),
                    MeshMaterial3d(material_handle.clone()),
                    Transform::from_translation(position).with_scale(Vec3::splat(cube_scale)),
                    BenchmarkEntity,
                ));
                if outline_enabled {
                    entity.insert(build_outline(width, outline_mode));
                }
                spawned += 1;
            }
        }
    }
}

struct GridSpawnSpec<'a> {
    count:           u32,
    width:           f32,
    cube_fill:       f32,
    viewport:        &'a ViewportInfo,
    outline_enabled: bool,
    outline_mode:    OutlineMethod,
}

// --- Main Benchmark Tick ---

#[derive(SystemParam)]
struct BenchmarkTickParams<'w, 's> {
    commands:           Commands<'w, 's>,
    state:              ResMut<'w, BenchmarkState>,
    meshes:             ResMut<'w, Assets<Mesh>>,
    materials:          ResMut<'w, Assets<StandardMaterial>>,
    time:               Res<'w, Time<Real>>,
    benchmark_entities: Query<'w, 's, Entity, With<BenchmarkEntity>>,
    camera_query:       Query<'w, 's, (&'static Transform, &'static Projection), With<Camera3d>>,
    windows:            Query<'w, 's, &'static mut Window>,
}

fn benchmark_tick(mut params: BenchmarkTickParams<'_, '_>) {
    match params.state.phase {
        BenchmarkPhase::Idle => {},
        BenchmarkPhase::StartupDelay => handle_startup_delay_phase(&mut params),
        BenchmarkPhase::Setup => handle_setup_phase(&mut params),
        BenchmarkPhase::Warmup => advance_warmup_phase(&mut params.state),
        BenchmarkPhase::Measure => measure_phase(&mut params.state, &params.time),
        BenchmarkPhase::Analyze => handle_analyze_phase(&mut params.state),
        BenchmarkPhase::ExitDelay => handle_exit_delay_phase(&mut params.state, &params.time),
    }
}

fn handle_startup_delay_phase(params: &mut BenchmarkTickParams<'_, '_>) {
    if params.state.startup_timer.elapsed_secs() == 0.0
        && let Ok(mut window) = params.windows.single_mut()
    {
        window.focused = true;
        info!("Auto mode: focusing window, waiting {AUTO_STARTUP_DELAY_SECS}s before starting");
    }

    params.state.startup_timer.tick(params.time.delta());
    if params.state.startup_timer.just_finished() {
        info!("Startup delay complete, beginning auto benchmark");
        params.state.phase = BenchmarkPhase::Setup;
    }
}

fn handle_setup_phase(params: &mut BenchmarkTickParams<'_, '_>) {
    for entity in &params.benchmark_entities {
        params.commands.entity(entity).despawn();
    }

    let result_name = params.state.result_name();
    params.state.results.retain(|r| r.name != result_name);

    let scenario = &SCENARIOS[params.state.current_scenario];
    let outline_label = if params.state.outline_enabled {
        "on"
    } else {
        "off"
    };
    info!(
        "Setting up scenario: {} [outline {outline_label}] ({}/{})",
        scenario.name,
        params.state.current_scenario + 1,
        SCENARIOS.len()
    );

    let Ok((camera_transform, projection)) = params.camera_query.single() else {
        return;
    };
    let Ok(window) = params.windows.single() else {
        return;
    };
    let viewport = compute_viewport_info(camera_transform, projection, window);

    spawn_scenario(
        &mut params.commands,
        &mut params.meshes,
        &mut params.materials,
        scenario,
        &viewport,
        params.state.outline_enabled,
        params.state.outline_mode,
    );

    params.state.frame_counter = 0;
    params.state.frame_times.clear();
    params.state.phase = BenchmarkPhase::Warmup;
}

const fn advance_warmup_phase(state: &mut BenchmarkState) {
    state.frame_counter += 1;
    if state.frame_counter >= WARMUP_FRAMES {
        state.frame_counter = 0;
        state.phase = BenchmarkPhase::Measure;
    }
}

fn measure_phase(state: &mut BenchmarkState, time: &Time<Real>) {
    let frame_time_ms = time.delta_secs_f64() * MS_PER_SECOND;
    state.frame_times.push(frame_time_ms);
    state.frame_counter += 1;

    if state.frame_counter >= MEASURE_FRAMES {
        state.phase = BenchmarkPhase::Analyze;
    }
}

fn handle_analyze_phase(state: &mut BenchmarkState) {
    let result_name = state.result_name();
    let result = compute_statistics(&result_name, &mut state.frame_times);
    info!(
        "  {} — avg: {:.2}ms, median: {:.2}ms, p95: {:.2}ms, ~{:.0} FPS",
        result.name,
        result.avg_ms,
        result.median_ms,
        result.p95_ms,
        result.avg_fps()
    );

    if let Some(existing) = state.results.iter_mut().find(|r| r.name == result.name) {
        *existing = result;
    } else {
        state.results.push(result);
    }

    if !state.outline_enabled {
        state.outline_enabled = true;
        state.phase = BenchmarkPhase::Setup;
        return;
    }

    if state.mode == BenchmarkMode::Auto && state.current_scenario + 1 < SCENARIOS.len() {
        state.outline_enabled = false;
        state.current_scenario += 1;
        state.phase = BenchmarkPhase::Setup;
        return;
    }

    state.outline_enabled = false;
    if state.mode == BenchmarkMode::Auto {
        write_results(&state.results);
        if state.exit_on_complete {
            info!("Auto benchmark complete, exiting in {AUTO_EXIT_DELAY_SECS}s");
            state.phase = BenchmarkPhase::ExitDelay;
        } else {
            info!("Auto benchmark complete");
            state.mode = BenchmarkMode::Interactive;
            state.phase = BenchmarkPhase::Idle;
        }
    } else {
        state.phase = BenchmarkPhase::Idle;
    }
}

fn handle_exit_delay_phase(state: &mut BenchmarkState, time: &Time<Real>) {
    state.exit_timer.tick(time.delta());
    if state.exit_timer.just_finished() {
        info!("Exiting");
        std::process::exit(0);
    }
}

// --- Input Handling ---

fn handle_input(input: Res<ButtonInput<KeyCode>>, mut state: ResMut<BenchmarkState>) {
    // Log results
    if input.just_pressed(KeyCode::KeyL) && !state.results.is_empty() {
        write_results(&state.results);
        return;
    }

    // Start an auto benchmark run
    if input.just_pressed(KeyCode::KeyR) {
        info!("Starting auto benchmark run");
        state.mode = BenchmarkMode::Auto;
        state.current_scenario = 0;
        state.outline_enabled = false;
        state.results.clear();
        state.phase = BenchmarkPhase::Setup;
        return;
    }

    // Cycle outline mode
    if input.just_pressed(KeyCode::KeyM) {
        let new_mode = next_outline_mode(state.outline_mode);
        info!("Outline mode: {}", outline_mode_label(new_mode));
        state.outline_mode = new_mode;
        state.mode = BenchmarkMode::Interactive;
        state.outline_enabled = false;
        state.phase = BenchmarkPhase::Setup;
        return;
    }

    // Scenario switching — switches to interactive mode if in auto
    for (idx, scenario) in SCENARIOS.iter().enumerate() {
        if input.just_pressed(scenario.key) {
            info!("Switching to scenario: {}", scenario.name);
            state.mode = BenchmarkMode::Interactive;
            state.current_scenario = idx;
            state.outline_enabled = false;
            state.phase = BenchmarkPhase::Setup;
            return;
        }
    }
}

// --- HUD ---

const fn key_to_char(key: KeyCode) -> char {
    match key {
        KeyCode::Digit0 => '0',
        KeyCode::Digit1 => '1',
        KeyCode::Digit2 => '2',
        KeyCode::Digit3 => '3',
        KeyCode::Digit4 => '4',
        KeyCode::Digit5 => '5',
        KeyCode::Digit6 => '6',
        KeyCode::Digit7 => '7',
        KeyCode::Digit8 => '8',
        KeyCode::Digit9 => '9',
        _ => '?',
    }
}

fn update_hud(
    state: Res<BenchmarkState>,
    diagnostics: Res<DiagnosticsStore>,
    mut text: Single<&mut Text, With<HudText>>,
    time: Res<Time>,
    mut hud_timer: ResMut<HudUpdateTimer>,
) {
    if !hud_timer.0.tick(time.delta()).just_finished() {
        return;
    }
    text.0 = build_hud_text(&state, &diagnostics);
}

struct LiveMetrics {
    fps:        f64,
    frame_time: f64,
}

fn build_hud_text(state: &BenchmarkState, diagnostics: &DiagnosticsStore) -> String {
    let scenario = &SCENARIOS[state.current_scenario];
    let mode_label = benchmark_mode_label(&state.mode);
    let phase_info = benchmark_phase_label(state);
    let progress = auto_progress_label(state);
    let col = results_label_width();
    let outline_mode_name = outline_mode_label(state.outline_mode);
    let live_metrics = live_metrics(diagnostics);
    let bench_stats = benchmark_stats_line(state.frame_times.as_slice(), col);

    let mut hud = format!(
        "[{mode_label}] {}{progress}  Mode: {outline_mode_name}\n{phase_info}\n\n{:<col$}FPS: {fps:<4.0}  Frame: {frame_time:.2}ms{bench_stats}",
        scenario.name,
        "Bevy:",
        fps = live_metrics.fps,
        frame_time = live_metrics.frame_time,
    );

    append_results_section(&mut hud, state, col, outline_mode_name);
    hud.push_str("\n\n#: Switch scenario  M: Cycle mode  R: Auto run  L: Log results");
    hud
}

const fn benchmark_mode_label(mode: &BenchmarkMode) -> &'static str {
    match mode {
        BenchmarkMode::Auto => "Auto",
        BenchmarkMode::Interactive => "Interactive",
    }
}

fn benchmark_phase_label(state: &BenchmarkState) -> String {
    match state.phase {
        BenchmarkPhase::Idle => "Idle".to_string(),
        BenchmarkPhase::StartupDelay => {
            let remaining = AUTO_STARTUP_DELAY_SECS - state.startup_timer.elapsed_secs();
            format!("Starting in {remaining:.0}s...")
        },
        BenchmarkPhase::Setup => "Setting up...".to_string(),
        BenchmarkPhase::Warmup => {
            format!("Warmup {}/{WARMUP_FRAMES}", state.frame_counter)
        },
        BenchmarkPhase::Measure => {
            format!("Measuring {}/{MEASURE_FRAMES}", state.frame_counter)
        },
        BenchmarkPhase::Analyze => "Analyzing...".to_string(),
        BenchmarkPhase::ExitDelay => {
            let remaining = AUTO_EXIT_DELAY_SECS - state.exit_timer.elapsed_secs();
            format!("Exiting in {remaining:.0}s...")
        },
    }
}

fn auto_progress_label(state: &BenchmarkState) -> String {
    if state.mode == BenchmarkMode::Auto {
        format!(" ({}/{})", state.current_scenario + 1, SCENARIOS.len())
    } else {
        String::new()
    }
}

fn results_label_width() -> usize {
    let mut max_label_len = "Bench:".len();
    for scenario in SCENARIOS {
        max_label_len = max_label_len.max(scenario.name.len() + 6);
    }
    max_label_len + 1
}

fn live_metrics(diagnostics: &DiagnosticsStore) -> LiveMetrics {
    LiveMetrics {
        fps: diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(bevy::diagnostic::Diagnostic::smoothed)
            .unwrap_or(0.0),
        frame_time: diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
            .and_then(bevy::diagnostic::Diagnostic::smoothed)
            .unwrap_or(0.0),
    }
}

fn benchmark_stats_line(frame_times: &[f64], col: usize) -> String {
    if frame_times.is_empty() {
        return String::new();
    }

    let sum: f64 = frame_times.iter().sum();
    let avg_ms = sum / frame_times.len().to_f64();
    let avg_fps = MS_PER_SECOND / avg_ms;
    format!("\n{:<col$}FPS: {avg_fps:<4.0}  Frame: {avg_ms:.2}ms", "Bench:")
}

fn append_results_section(
    hud: &mut String,
    state: &BenchmarkState,
    col: usize,
    outline_mode_name: &str,
) {
    hud.push_str("\n\n--- Results ---");
    for scenario in SCENARIOS {
        append_scenario_results(hud, state, scenario, col, outline_mode_name);
    }
}

fn append_scenario_results(
    hud: &mut String,
    state: &BenchmarkState,
    scenario: &ScenarioDefinition,
    col: usize,
    outline_mode_name: &str,
) {
    let key_char = key_to_char(scenario.key);
    for (index, suffix) in ["off", "on"].iter().enumerate() {
        let result_name = format!("{} {suffix} ({outline_mode_name})", scenario.name);
        let label = if index == 0 {
            format!("{key_char} {result_name}:")
        } else {
            format!("  {result_name}:")
        };
        append_result_row(hud, state, &result_name, &label, col);
    }
}

fn append_result_row(
    hud: &mut String,
    state: &BenchmarkState,
    result_name: &str,
    label: &str,
    col: usize,
) {
    if let Some(result) = state.results.iter().find(|r| r.name == result_name) {
        let _ = write!(
            hud,
            "\n{label:<col$}FPS: {:<4.0}  Frame: {:.2}ms  med: {:.2}ms  p95: {:.2}ms",
            result.avg_fps(),
            result.avg_ms,
            result.median_ms,
            result.p95_ms,
        );
    } else {
        let _ = write!(hud, "\n{label:<col$}---");
    }
}

// --- Statistics ---

fn compute_statistics(name: &str, frame_times: &mut [f64]) -> ScenarioResult {
    frame_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = frame_times.len();
    let sum: f64 = frame_times.iter().sum();
    let avg_ms = sum / len.to_f64();
    let median_ms = percentile(frame_times, 50.0);
    let p95_ms = percentile(frame_times, 95.0);
    let p99_ms = percentile(frame_times, 99.0);
    let min_ms = frame_times.first().copied().unwrap_or(0.0);
    let max_ms = frame_times.last().copied().unwrap_or(0.0);

    ScenarioResult {
        name: (*name).to_string(),
        frames: len.to_u32(),
        avg_ms,
        median_ms,
        p95_ms,
        p99_ms,
        min_ms,
        max_ms,
    }
}

fn percentile(sorted: &[f64], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let len_f64 = (sorted.len() - 1).to_f64();
    let idx = (pct / 100.0 * len_f64).round().to_u32().to_usize();
    sorted[idx.min(sorted.len() - 1)]
}

// --- Results Output ---

fn write_results(results: &[ScenarioResult]) {
    let mut table = String::new();
    let _ = writeln!(table, "\n=== bevy_liminal Benchmark Results ===\n");
    let _ = writeln!(
        table,
        "{:<18}| {:>6} | {:>8} | {:>8} | {:>8} | {:>8} | {:>8} | {:>8} | {:>6}",
        "Scenario",
        "Frames",
        "Avg(ms)",
        "Med(ms)",
        "P95(ms)",
        "P99(ms)",
        "Min(ms)",
        "Max(ms)",
        "~FPS"
    );
    let _ = writeln!(
        table,
        "{:-<18}|{:->8}|{:->10}|{:->10}|{:->10}|{:->10}|{:->10}|{:->10}|{:->8}",
        "", "", "", "", "", "", "", "", ""
    );

    for r in results {
        let _ = writeln!(
            table,
            "{:<18}| {:>6} | {:>8.2} | {:>8.2} | {:>8.2} | {:>8.2} | {:>8.2} | {:>8.2} | {:>6.0}",
            r.name,
            r.frames,
            r.avg_ms,
            r.median_ms,
            r.p95_ms,
            r.p99_ms,
            r.min_ms,
            r.max_ms,
            r.avg_fps()
        );
    }

    info!("{table}");

    // Write CSV
    match write_csv(results) {
        Ok(path) => info!("CSV written to: {path}"),
        Err(e) => warn!("Failed to write CSV: {e}"),
    }
}

fn format_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let output = std::process::Command::new("date")
        .args(["-r", &now.to_string(), "+%Y_%m_%d_%H_%M"])
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => format!("{now}"),
    }
}

fn write_csv(results: &[ScenarioResult]) -> Result<String, std::io::Error> {
    let results_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("results");
    std::fs::create_dir_all(&results_dir)?;

    let timestamp = format_timestamp();
    let path = results_dir.join(format!("benchmark_{timestamp}.csv"));
    let mut file = File::create(&path)?;
    writeln!(
        file,
        "scenario,frames,avg_ms,median_ms,p95_ms,p99_ms,min_ms,max_ms,avg_fps"
    )?;
    for r in results {
        writeln!(
            file,
            "{},{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.0}",
            r.name,
            r.frames,
            r.avg_ms,
            r.median_ms,
            r.p95_ms,
            r.p99_ms,
            r.min_ms,
            r.max_ms,
            r.avg_fps()
        )?;
    }
    Ok(path.display().to_string())
}
