//! Side-by-side comparison of outline methods and overlap modes.

use std::f32::consts::PI;

use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::GREEN;
use bevy::color::palettes::css::RED;
use bevy::color::palettes::css::SILVER;
use bevy::color::palettes::css::YELLOW;
use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_brp_extras::PortDisplay;
use bevy_lagrange::InputControl;
use bevy_lagrange::LagrangePlugin;
use bevy_lagrange::OrbitCam;
use bevy_lagrange::TrackpadInput;
use bevy_liminal::LiminalPlugin;
use bevy_liminal::Outline;
use bevy_liminal::OutlineCamera;
use bevy_liminal::OutlineMethod;
use bevy_liminal::OverlapMode;
use bevy_window_manager::WindowManagerPlugin;

// Initial overlap modes
const INITIAL_HULL_OVERLAP: OverlapMode = OverlapMode::Merged;
const INITIAL_SHELL_OVERLAP: OverlapMode = OverlapMode::PerMesh;

// Initial widths
const INITIAL_HULL_WIDTH_WORLD: f32 = 0.01;
const INITIAL_JUMP_FLOOD_WIDTH_PX: f32 = 5.0;
const INITIAL_SHELL_WIDTH_PX: f32 = 2.0;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "outline_mode - outline mode comparison".into(),
                        ..default()
                    }),
                    ..default()
                }),
            BrpExtrasPlugin::default().port_in_title(PortDisplay::NonDefault),
            LagrangePlugin,
            LiminalPlugin,
            WindowManagerPlugin,
        ))
        .init_resource::<OutlineModeToggle>()
        .init_resource::<OutlineWidthControl>()
        .init_resource::<OverlapControl>()
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(
            Update,
            (
                (toggle_outline_mode, adjust_outline_width, adjust_overlap),
                update_ui,
            ),
        )
        .run();
}

#[derive(Resource)]
struct OutlineModeToggle {
    outline_method: OutlineMethod,
}

impl Default for OutlineModeToggle {
    fn default() -> Self {
        Self {
            outline_method: OutlineMethod::WorldHull,
        }
    }
}

#[derive(Resource)]
struct OutlineWidthControl {
    jump_flood_width_px: f32,
    hull_width_world:    f32,
    shell_width_px:      f32,
}

impl Default for OutlineWidthControl {
    fn default() -> Self {
        Self {
            jump_flood_width_px: INITIAL_JUMP_FLOOD_WIDTH_PX,
            hull_width_world:    INITIAL_HULL_WIDTH_WORLD,
            shell_width_px:      INITIAL_SHELL_WIDTH_PX,
        }
    }
}

#[derive(Resource)]
struct OverlapControl {
    hull_overlap:  OverlapMode,
    shell_overlap: OverlapMode,
}

impl Default for OverlapControl {
    fn default() -> Self {
        Self {
            hull_overlap:  INITIAL_HULL_OVERLAP,
            shell_overlap: INITIAL_SHELL_OVERLAP,
        }
    }
}

#[derive(Component)]
struct StatusText;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(2.2, 1.2, 2.2).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        OrbitCam {
            button_orbit: MouseButton::Middle,
            button_pan: MouseButton::Middle,
            modifier_pan: Some(KeyCode::ShiftLeft),
            input_control: Some(InputControl {
                trackpad: Some(TrackpadInput::blender_default()),
                ..default()
            }),
            orbit_smoothness: 0.0,
            pan_smoothness: 0.0,
            zoom_smoothness: 0.0,
            focus: Vec3::new(0.0, 1.0, 0.0),
            radius: Some(2.8),
            ..default()
        },
        OutlineCamera,
    ));

    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(materials.add(Color::from(SILVER))),
    ));

    // Intersecting pair: yellow cube (transparent) and blue sphere
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::from(YELLOW).with_alpha(0.5),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(0.0, 1.0, 0.0)
            .with_rotation(Quat::from_rotation_x(PI / 5.0) * Quat::from_rotation_y(PI / 3.0)),
        Outline::world_hull(INITIAL_HULL_WIDTH_WORLD)
            .with_color(Color::from(RED))
            .with_overlap(INITIAL_HULL_OVERLAP)
            .build(),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::default())),
        MeshMaterial3d(materials.add(Color::from(BLUE))),
        Transform::from_xyz(-0.5, 1.0, 0.5),
        Outline::world_hull(INITIAL_HULL_WIDTH_WORLD)
            .with_color(Color::from(GREEN))
            .with_intensity(10.0)
            .with_overlap(INITIAL_HULL_OVERLAP)
            .build(),
    ));

    // Non-intersecting pair: cube in front of sphere (screen overlap only)
    let non_intersect_cube_mat = materials.add(StandardMaterial {
        base_color: Color::from(YELLOW).with_alpha(0.5),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let non_intersect_sphere_mat = materials.add(Color::from(BLUE));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(non_intersect_sphere_mat),
        Transform::from_xyz(-0.75, 1.0, -7.8),
        Outline::world_hull(INITIAL_HULL_WIDTH_WORLD)
            .with_color(Color::from(GREEN))
            .with_intensity(10.0)
            .with_overlap(INITIAL_HULL_OVERLAP)
            .build(),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.6, 0.6, 0.6))),
        MeshMaterial3d(non_intersect_cube_mat),
        Transform::from_xyz(0.0, 1.0, -4.0)
            .with_rotation(Quat::from_rotation_x(PI / 5.0) * Quat::from_rotation_y(PI / 3.0)),
        Outline::world_hull(INITIAL_HULL_WIDTH_WORLD)
            .with_color(Color::from(RED))
            .with_overlap(INITIAL_HULL_OVERLAP)
            .build(),
    ));
}

fn setup_ui(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        StatusText,
    ));
}

fn toggle_outline_mode(
    input: Res<ButtonInput<KeyCode>>,
    width_control: Res<OutlineWidthControl>,
    overlap_control: Res<OverlapControl>,
    mut mode_toggle: ResMut<OutlineModeToggle>,
    mut outline_query: Query<&mut Outline>,
) {
    if !input.just_pressed(KeyCode::KeyM) {
        return;
    }

    mode_toggle.outline_method = match mode_toggle.outline_method {
        OutlineMethod::JumpFlood => OutlineMethod::WorldHull,
        OutlineMethod::WorldHull => OutlineMethod::ScreenHull,
        OutlineMethod::ScreenHull => OutlineMethod::JumpFlood,
    };

    let (width, overlap) = match mode_toggle.outline_method {
        OutlineMethod::JumpFlood => (width_control.jump_flood_width_px, OverlapMode::Merged),
        OutlineMethod::WorldHull => (width_control.hull_width_world, overlap_control.hull_overlap),
        OutlineMethod::ScreenHull => (width_control.shell_width_px, overlap_control.shell_overlap),
    };

    for mut outline in &mut outline_query {
        *outline = rebuilt_outline_for_mode(&outline, mode_toggle.outline_method, width, overlap);
    }
}

const fn rebuilt_outline_for_mode(
    current: &Outline,
    mode: OutlineMethod,
    width: f32,
    overlap: OverlapMode,
) -> Outline {
    match mode {
        OutlineMethod::JumpFlood => Outline::jump_flood(width)
            .with_intensity(current.intensity)
            .with_color(current.color)
            .build(),
        OutlineMethod::WorldHull => Outline::world_hull(width)
            .with_intensity(current.intensity)
            .with_color(current.color)
            .with_overlap(overlap)
            .build(),
        OutlineMethod::ScreenHull => Outline::screen_hull(width)
            .with_intensity(current.intensity)
            .with_color(current.color)
            .with_overlap(overlap)
            .build(),
    }
}

fn adjust_outline_width(
    input: Res<ButtonInput<KeyCode>>,
    mode_toggle: Res<OutlineModeToggle>,
    mut width_control: ResMut<OutlineWidthControl>,
    mut outline_query: Query<&mut Outline>,
) {
    let decrease = input.just_pressed(KeyCode::ArrowLeft);
    let increase = input.just_pressed(KeyCode::ArrowRight);
    if !decrease && !increase {
        return;
    }

    match mode_toggle.outline_method {
        OutlineMethod::JumpFlood => {
            let mut next = width_control.jump_flood_width_px;
            if decrease {
                next = (next - 1.0).max(1.0);
            }
            if increase {
                next += 1.0;
            }
            width_control.jump_flood_width_px = next;
            for mut outline in &mut outline_query {
                outline.width = next;
            }
        },
        OutlineMethod::WorldHull => {
            let mut next = width_control.hull_width_world;
            if decrease {
                next /= 1.2;
            }
            if increase {
                next *= 1.2;
            }
            width_control.hull_width_world = next.clamp(0.0001, 10.0);
            for mut outline in &mut outline_query {
                outline.width = width_control.hull_width_world;
            }
        },
        OutlineMethod::ScreenHull => {
            let mut next = width_control.shell_width_px;
            if decrease {
                next = (next - 0.5).max(0.5);
            }
            if increase {
                next += 0.5;
            }
            width_control.shell_width_px = next;
            for mut outline in &mut outline_query {
                outline.width = next;
            }
        },
    }
}

fn adjust_overlap(
    input: Res<ButtonInput<KeyCode>>,
    mode_toggle: Res<OutlineModeToggle>,
    mut overlap_control: ResMut<OverlapControl>,
    mut outline_query: Query<&mut Outline>,
) {
    let decrease = input.just_pressed(KeyCode::Minus);
    let increase = input.just_pressed(KeyCode::Equal);
    if !decrease && !increase {
        return;
    }

    let Some(current) = (match mode_toggle.outline_method {
        OutlineMethod::WorldHull => Some(&mut overlap_control.hull_overlap),
        OutlineMethod::ScreenHull => Some(&mut overlap_control.shell_overlap),
        OutlineMethod::JumpFlood => None,
    }) else {
        return;
    };

    *current = match *current {
        OverlapMode::Merged => OverlapMode::PerMesh,
        OverlapMode::PerMesh | OverlapMode::Grouped => OverlapMode::Merged,
    };

    let value = *current;
    for mut outline in &mut outline_query {
        outline.overlap = value;
    }
}

fn update_ui(
    mode_toggle: Res<OutlineModeToggle>,
    width_control: Res<OutlineWidthControl>,
    overlap_control: Res<OverlapControl>,
    mut text_query: Single<&mut Text, With<StatusText>>,
) {
    let mode_line = match mode_toggle.outline_method {
        OutlineMethod::JumpFlood => "Mode: JumpFlood (M)",
        OutlineMethod::WorldHull => "Mode: WorldHull (M)",
        OutlineMethod::ScreenHull => "Mode: ScreenHull (M)",
    };

    let width_line = match mode_toggle.outline_method {
        OutlineMethod::JumpFlood => {
            format!(
                "Width: {:.1} px (Left / Right)",
                width_control.jump_flood_width_px
            )
        },
        OutlineMethod::WorldHull => {
            format!(
                "Width: {:.4} m (Left / Right)",
                width_control.hull_width_world
            )
        },
        OutlineMethod::ScreenHull => {
            format!(
                "Width: {:.1} px (Left / Right)",
                width_control.shell_width_px
            )
        },
    };

    let overlap_line = match mode_toggle.outline_method {
        OutlineMethod::JumpFlood => String::new(),
        OutlineMethod::WorldHull => {
            format!(
                "Overlap: {} (- / +)",
                overlap_mode_label(overlap_control.hull_overlap)
            )
        },
        OutlineMethod::ScreenHull => {
            format!(
                "Overlap: {} (- / +)",
                overlap_mode_label(overlap_control.shell_overlap)
            )
        },
    };

    text_query.0 = format!("{mode_line}\n{width_line}\n{overlap_line}");
}

const fn overlap_mode_label(mode: OverlapMode) -> &'static str {
    match mode {
        OverlapMode::Merged => "Merged",
        OverlapMode::PerMesh => "PerMesh",
        OverlapMode::Grouped => "Grouped",
    }
}
