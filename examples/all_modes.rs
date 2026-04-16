//! @generated `bevy_example_template`
//! Interactive gallery of all outline methods applied to various mesh types.

use std::time::Duration;

use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_brp_extras::PortDisplay;
use bevy_kana::ToF32;
use bevy_lagrange::InputControl;
use bevy_lagrange::LagrangePlugin;
use bevy_lagrange::OrbitCam;
use bevy_lagrange::TrackpadInput;
use bevy_lagrange::ZoomToFit;
use bevy_liminal::LiminalPlugin;
use bevy_liminal::Outline;
use bevy_liminal::OutlineCamera;
use bevy_liminal::OutlineMethod;
use bevy_liminal::OverlapMode;
use bevy_window_manager::WindowManagerPlugin;

// Grid layout
const GRID_SPACING: f32 = 5.0;
const SPACESHIP_SCALE: f32 = 0.3;

// Outline
const OUTLINE_COLOR: Color = Color::srgb(0.0, 0.8, 1.0);
const OUTLINE_INTENSITY: f32 = 1.5;
const OUTLINE_WIDTH: f32 = 4.0;

// Zoom
const ZOOM_DURATION_MS: u64 = 1000;
const ZOOM_MARGIN_MESH: f32 = 0.15;
const ZOOM_MARGIN_SCENE: f32 = 0.08;

struct MeshAndMaterial {
    mesh:     Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Resource)]
struct SceneBounds(Entity);

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            LagrangePlugin,
            BrpExtrasPlugin::default().port_in_title(PortDisplay::NonDefault),
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
    let cube = MeshAndMaterial {
        mesh:     meshes.add(Cuboid::default()),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.7, 0.6),
            ..default()
        }),
    };
    let sphere = MeshAndMaterial {
        mesh:     meshes.add(Sphere::new(0.25).mesh().uv(32, 16)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.65, 0.55, 0.75),
            ..default()
        }),
    };
    let torus = MeshAndMaterial {
        mesh:     meshes.add(
            Torus::new(0.25, 0.75)
                .mesh()
                .minor_resolution(64)
                .major_resolution(64),
        ),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.7, 0.3),
            ..default()
        }),
    };

    spawn_outline_grid(&mut commands, &cube, &sphere, &torus, &asset_server);
    spawn_environment(&mut commands, &mut meshes, &mut materials);
    spawn_ui(&mut commands);
}

fn spawn_outline_grid(
    commands: &mut Commands,
    cube: &MeshAndMaterial,
    sphere: &MeshAndMaterial,
    torus: &MeshAndMaterial,
    asset_server: &AssetServer,
) {
    let modes: &[(OutlineMethod, &str)] = &[
        (OutlineMethod::WorldHull, "WorldHull"),
        (OutlineMethod::ScreenHull, "ScreenHull"),
        (OutlineMethod::JumpFlood, "JumpFlood"),
    ];

    let column_rotations = [
        Quat::from_euler(EulerRot::YXZ, 0.7, 0.4, 0.0),
        Quat::IDENTITY,
        Quat::from_euler(EulerRot::YXZ, -0.7, -0.9, 0.15),
    ];

    for (col, &(mode, label)) in modes.iter().enumerate() {
        let x = (col.to_f32() - 1.0) * GRID_SPACING;
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
        };

        commands
            .spawn((
                Name::new(format!("Torus ({label})")),
                Mesh3d(torus.mesh.clone()),
                MeshMaterial3d(torus.material.clone()),
                Transform {
                    translation: Vec3::new(x, 1.0, -GRID_SPACING),
                    rotation,
                    ..default()
                },
                outline.clone(),
            ))
            .observe(on_mesh_clicked);

        commands
            .spawn((
                Name::new(format!("Cube ({label})")),
                Mesh3d(cube.mesh.clone()),
                MeshMaterial3d(cube.material.clone()),
                Transform {
                    translation: Vec3::new(x, 1.0, 0.0),
                    rotation,
                    ..default()
                },
                outline.clone(),
            ))
            .observe(on_mesh_clicked)
            .with_children(|parent| {
                parent.spawn((
                    Name::new("Sphere +X"),
                    Mesh3d(sphere.mesh.clone()),
                    MeshMaterial3d(sphere.material.clone()),
                    Transform::from_xyz(0.5, 0.0, 0.0),
                ));
                parent.spawn((
                    Name::new("Sphere -X"),
                    Mesh3d(sphere.mesh.clone()),
                    MeshMaterial3d(sphere.material.clone()),
                    Transform::from_xyz(-0.5, 0.0, 0.0),
                ));
            });

        commands
            .spawn((
                Name::new(format!("Spaceship ({label})")),
                SceneRoot(asset_server.load("spaceship.glb#Scene0")),
                Transform {
                    translation: Vec3::new(x, 1.5, GRID_SPACING),
                    rotation,
                    scale: Vec3::splat(SPACESHIP_SCALE),
                },
                outline.clone(),
            ))
            .observe(on_mesh_clicked);

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
}

fn spawn_environment(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
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

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        OutlineCamera,
        OrbitCam {
            button_orbit: MouseButton::Middle,
            button_pan: MouseButton::Middle,
            modifier_pan: Some(KeyCode::ShiftLeft),
            input_control: Some(InputControl {
                trackpad: Some(TrackpadInput::blender_default()),
                ..default()
            }),
            ..default()
        },
        Transform::from_xyz(0.0, 12.0, 18.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn spawn_ui(commands: &mut Commands) {
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
        };
        outline.overlap = toggled;
        new_mode = Some(toggled);
    }

    if let Some(mode) = new_mode
        && let Ok(mut text) = label.single_mut()
    {
        let label_str = match mode {
            OverlapMode::Merged => "Merged",
            OverlapMode::Grouped => "Grouped",
            OverlapMode::PerMesh => "PerMesh",
        };
        **text = format!("Overlap: {label_str}");
    }
}
