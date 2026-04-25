use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::constants::AUTO_EXIT_DELAY_SECS;
use crate::constants::AUTO_STARTUP_DELAY_SECS;
use crate::constants::MEASURE_FRAMES;
use crate::constants::MS_PER_SECOND;
use crate::constants::WARMUP_FRAMES;
use crate::results::compute_statistics;
use crate::results::write_results;
use crate::scenario::BenchmarkEntity;
use crate::scenario::SCENARIOS;
use crate::scenario::compute_viewport_info;
use crate::scenario::spawn_scenario;
use crate::state::BenchmarkMode;
use crate::state::BenchmarkPhase;
use crate::state::BenchmarkState;
use crate::state::OutlinePresence;
use crate::state::next_outline_method;
use crate::state::outline_method_label;

#[derive(SystemParam)]
pub(super) struct BenchmarkTickParams<'w, 's> {
    commands:           Commands<'w, 's>,
    state:              ResMut<'w, BenchmarkState>,
    meshes:             ResMut<'w, Assets<Mesh>>,
    materials:          ResMut<'w, Assets<StandardMaterial>>,
    time:               Res<'w, Time<Real>>,
    benchmark_entities: Query<'w, 's, Entity, With<BenchmarkEntity>>,
    camera_query:       Query<'w, 's, (&'static Transform, &'static Projection), With<Camera3d>>,
    windows:            Query<'w, 's, &'static mut Window>,
}

pub(super) fn benchmark_tick(mut params: BenchmarkTickParams<'_, '_>) {
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
    params
        .state
        .results
        .retain(|result| result.name != result_name);

    let scenario = &SCENARIOS[params.state.current_scenario];
    let outline_label = match params.state.outline_presence {
        OutlinePresence::Enabled => "on",
        OutlinePresence::Disabled => "off",
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
        params.state.outline_presence,
        params.state.outline_method,
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
        result.avg,
        result.median,
        result.p95,
        result.avg_fps()
    );

    if let Some(existing) = state
        .results
        .iter_mut()
        .find(|existing| existing.name == result.name)
    {
        *existing = result;
    } else {
        state.results.push(result);
    }

    if state.outline_presence == OutlinePresence::Disabled {
        state.outline_presence = OutlinePresence::Enabled;
        state.phase = BenchmarkPhase::Setup;
        return;
    }

    if state.benchmark_mode == BenchmarkMode::Auto && state.current_scenario + 1 < SCENARIOS.len() {
        state.outline_presence = OutlinePresence::Disabled;
        state.current_scenario += 1;
        state.phase = BenchmarkPhase::Setup;
        return;
    }

    state.outline_presence = OutlinePresence::Disabled;
    if state.benchmark_mode == BenchmarkMode::Auto {
        write_results(&state.results);
        if state.exit_behavior == crate::state::ExitBehavior::OnComplete {
            info!("Auto benchmark complete, exiting in {AUTO_EXIT_DELAY_SECS}s");
            state.phase = BenchmarkPhase::ExitDelay;
        } else {
            info!("Auto benchmark complete");
            state.benchmark_mode = BenchmarkMode::Interactive;
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

pub(super) fn handle_input(input: Res<ButtonInput<KeyCode>>, mut state: ResMut<BenchmarkState>) {
    if input.just_pressed(KeyCode::KeyL) && !state.results.is_empty() {
        write_results(&state.results);
        return;
    }

    if input.just_pressed(KeyCode::KeyR) {
        info!("Starting auto benchmark run");
        state.benchmark_mode = BenchmarkMode::Auto;
        state.current_scenario = 0;
        state.outline_presence = OutlinePresence::Disabled;
        state.results.clear();
        state.phase = BenchmarkPhase::Setup;
        return;
    }

    if input.just_pressed(KeyCode::KeyM) {
        let new_outline_method = next_outline_method(state.outline_method);
        info!("Outline mode: {}", outline_method_label(new_outline_method));
        state.outline_method = new_outline_method;
        state.benchmark_mode = BenchmarkMode::Interactive;
        state.outline_presence = OutlinePresence::Disabled;
        state.phase = BenchmarkPhase::Setup;
        return;
    }

    for (index, scenario) in SCENARIOS.iter().enumerate() {
        if input.just_pressed(scenario.key) {
            info!("Switching to scenario: {}", scenario.name);
            state.benchmark_mode = BenchmarkMode::Interactive;
            state.current_scenario = index;
            state.outline_presence = OutlinePresence::Disabled;
            state.phase = BenchmarkPhase::Setup;
            return;
        }
    }
}
