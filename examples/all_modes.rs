use std::time::Duration;

use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::prelude::*;
use bevy::scene::SceneInstanceReady;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_liminal::LiminalPlugin;
use bevy_liminal::Outline;
use bevy_liminal::OutlineCamera;
use bevy_liminal::OutlineMethod;
use bevy_liminal::OverlapMode;
use bevy_panorbit_camera::PanOrbitCamera;
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use bevy_panorbit_camera::TrackpadBehavior;
use bevy_panorbit_camera_ext::PanOrbitCameraExtPlugin;
use bevy_panorbit_camera_ext::ZoomToFit;
use bevy_window_manager::WindowManagerPlugin;

const OUTLINE_WIDTH: f32 = 4.0;
const OUTLINE_COLOR: Color = Color::srgb(0.0, 0.8, 1.0);
const OUTLINE_INTENSITY: f32 = 1.5;
const GRID_SPACING: f32 = 5.0;
const SPACESHIP_SCALE: f32 = 0.3;
const ZOOM_MARGIN_MESH: f32 = 0.15;
const ZOOM_MARGIN_SCENE: f32 = 0.08;
const ZOOM_DURATION_MS: u64 = 1000;

#[derive(Resource)]
struct SceneBounds(Entity);

#[derive(Component)]
struct ApplyOutlineOnReady {
    outline: Outline,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PanOrbitCameraPlugin,
            PanOrbitCameraExtPlugin,
            BrpExtrasPlugin::default(),
            WindowManagerPlugin,
            MeshPickingPlugin,
            LiminalPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, toggle_overlap)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let cube_mesh = meshes.add(Cuboid::default());
    let cube_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.7, 0.6),
        ..default()
    });
    let sphere_mesh = meshes.add(Sphere::new(0.25).mesh().uv(32, 16));
    let sphere_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.55, 0.75),
        ..default()
    });

    let torus_mesh = meshes.add(
        Torus::new(0.25, 0.75)
            .mesh()
            .minor_resolution(64)
            .major_resolution(64),
    );
    let torus_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.7, 0.3),
        ..default()
    });

    let modes: &[(OutlineMethod, &str)] = &[
        (OutlineMethod::WorldHull, "WorldHull"),
        (OutlineMethod::ScreenHull, "ScreenHull"),
        (OutlineMethod::JumpFlood, "JumpFlood"),
    ];

    // Per-column rotations shared by all shapes in that column.
    // Left: rotated toward camera and left — shows front and left faces.
    // Center: default orientation — straight-on view.
    // Right: rotated away from camera and right — shows back and right faces.
    let column_rotations = [
        Quat::from_euler(EulerRot::YXZ, 0.7, 0.4, 0.0),
        Quat::IDENTITY,
        Quat::from_euler(EulerRot::YXZ, -0.7, -0.9, 0.15),
    ];

    // 3x3 grid: rows are back→front (torus, cube, spaceship), columns are outline methods
    for (col, &(mode, label)) in modes.iter().enumerate() {
        let x = (col as f32 - 1.0) * GRID_SPACING;
        let rotation = column_rotations[col];
        let outline = match mode {
            OutlineMethod::JumpFlood => Outline::jump_flood(OUTLINE_WIDTH)
                .with_color(OUTLINE_COLOR)
                .with_intensity(OUTLINE_INTENSITY)
                .build(),
            OutlineMethod::WorldHull => Outline::world_hull(0.03)
                .with_color(OUTLINE_COLOR)
                .with_intensity(OUTLINE_INTENSITY)
                .build(),
            OutlineMethod::ScreenHull => Outline::screen_hull(OUTLINE_WIDTH)
                .with_color(OUTLINE_COLOR)
                .with_intensity(OUTLINE_INTENSITY)
                .build(),
            _ => unreachable!(),
        };

        // Back row: torus
        commands
            .spawn((
                Name::new(format!("Torus ({label})")),
                Mesh3d(torus_mesh.clone()),
                MeshMaterial3d(torus_material.clone()),
                Transform {
                    translation: Vec3::new(x, 1.0, -GRID_SPACING),
                    rotation,
                    ..default()
                },
                outline.clone(),
            ))
            .observe(on_mesh_clicked);

        // Middle row: cube with child spheres on opposite faces.
        // Default overlap is `Merged`; the toggle cycles through all 3 modes
        // and updates `group_owner` on children to demonstrate `Grouped`.
        let mut cube_outline = outline.clone();
        cube_outline.overlap = OverlapMode::Grouped;
        let cube_entity = commands
            .spawn((
                Name::new(format!("Cube ({label})")),
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(cube_material.clone()),
                Transform {
                    translation: Vec3::new(x, 1.0, 0.0),
                    rotation,
                    ..default()
                },
                cube_outline.clone(),
            ))
            .observe(on_mesh_clicked)
            .id();

        let mut sphere_outline = cube_outline.clone();
        sphere_outline.group_owner = Some(cube_entity);
        // Sphere on +X face
        let sphere_pos = commands
            .spawn((
                Name::new("Sphere +X"),
                Mesh3d(sphere_mesh.clone()),
                MeshMaterial3d(sphere_material.clone()),
                Transform::from_xyz(0.5, 0.0, 0.0),
                sphere_outline.clone(),
            ))
            .id();
        // Sphere on -X face
        let sphere_neg = commands
            .spawn((
                Name::new("Sphere -X"),
                Mesh3d(sphere_mesh.clone()),
                MeshMaterial3d(sphere_material.clone()),
                Transform::from_xyz(-0.5, 0.0, 0.0),
                sphere_outline.clone(),
            ))
            .id();
        commands
            .entity(cube_entity)
            .add_children(&[sphere_pos, sphere_neg]);

        // Front row: spaceship
        commands
            .spawn((
                Name::new(format!("Spaceship ({label})")),
                SceneRoot(asset_server.load("spaceship.glb#Scene0")),
                Transform {
                    translation: Vec3::new(x, 1.5, GRID_SPACING),
                    rotation,
                    scale: Vec3::splat(SPACESHIP_SCALE),
                },
                ApplyOutlineOnReady {
                    outline: outline.clone(),
                },
            ))
            .observe(on_scene_ready)
            .observe(on_mesh_clicked);

        // Column label
        commands.spawn((
            Text2d::new(label),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(x * 80.0, 280.0, 0.0),
        ));
    }

    // Ground plane
    let ground = commands
        .spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(18.0, 18.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.5, 0.3),
                double_sided: true,
                cull_mode: None,
                ..default()
            })),
        ))
        .observe(on_ground_clicked)
        .id();

    commands.insert_resource(SceneBounds(ground));

    // Light
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Camera
    commands.spawn((
        OutlineCamera,
        PanOrbitCamera {
            button_orbit: MouseButton::Middle,
            button_pan: MouseButton::Middle,
            modifier_pan: Some(KeyCode::ShiftLeft),
            trackpad_behavior: TrackpadBehavior::BlenderLike {
                modifier_pan:  Some(KeyCode::ShiftLeft),
                modifier_zoom: Some(KeyCode::ControlLeft),
            },
            trackpad_pinch_to_zoom_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 12.0, 18.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Instructions
    commands.spawn((
        Text::new(
            "Click a mesh to zoom-to-fit\n\
             Click the ground to zoom back out\n\
             Press 'O' to toggle overlap mode\n\
             \n\
             Columns: WorldHull | ScreenHull | JumpFlood\n\
             Rows: Torus | Cube | Spaceship",
        ),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    // Overlap mode label (bottom-left)
    commands.spawn((
        OverlapLabel,
        Text::new("Overlap: Merged"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 0.5, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

fn on_scene_ready(
    ready: On<SceneInstanceReady>,
    parent_query: Query<&ApplyOutlineOnReady>,
    mesh_query: Query<(), With<Mesh3d>>,
    children_query: Query<&Children>,
    mut commands: Commands,
) {
    let Ok(apply) = parent_query.get(ready.entity) else {
        return;
    };
    let outline = apply.outline.clone();

    for descendant in children_query.iter_descendants(ready.entity) {
        if mesh_query.contains(descendant) {
            commands.entity(descendant).insert(outline.clone());
        }
    }
}

fn on_mesh_clicked(click: On<Pointer<Click>>, mut commands: Commands) {
    if click.button != PointerButton::Primary {
        return;
    }
    info!("Mesh clicked: {:?}", click.entity);
    let camera = click.hit.camera;
    commands.trigger(
        ZoomToFit::new(camera, click.entity)
            .margin(ZOOM_MARGIN_MESH)
            .duration(Duration::from_millis(ZOOM_DURATION_MS)),
    );
}

fn on_ground_clicked(click: On<Pointer<Click>>, mut commands: Commands, scene: Res<SceneBounds>) {
    if click.button != PointerButton::Primary {
        return;
    }
    info!("Ground clicked, zooming to scene bounds");
    let camera = click.hit.camera;
    commands.trigger(
        ZoomToFit::new(camera, scene.0)
            .margin(ZOOM_MARGIN_SCENE)
            .duration(Duration::from_millis(ZOOM_DURATION_MS)),
    );
}

#[derive(Component)]
struct OverlapLabel;

fn toggle_overlap(
    keys: Res<ButtonInput<KeyCode>>,
    mut outlines: Query<&mut Outline>,
    mut label: Query<&mut Text, With<OverlapLabel>>,
) {
    if !keys.just_pressed(KeyCode::KeyO) {
        return;
    }

    let mut new_mode = None;
    for mut outline in &mut outlines {
        let toggled = match outline.overlap {
            OverlapMode::Merged => OverlapMode::Grouped,
            OverlapMode::Grouped => OverlapMode::PerMesh,
            OverlapMode::PerMesh => OverlapMode::Merged,
            _ => outline.overlap,
        };
        outline.overlap = toggled;
        new_mode = Some(toggled);
    }

    if let Some(mode) = new_mode {
        if let Ok(mut text) = label.single_mut() {
            let label_str = match mode {
                OverlapMode::Merged => "Merged",
                OverlapMode::Grouped => "Grouped",
                OverlapMode::PerMesh => "PerMesh",
                _ => "Unknown",
            };
            **text = format!("Overlap: {label_str}");
        }
    }
}
