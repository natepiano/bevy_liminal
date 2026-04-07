//! Outlines with different anti-aliasing modes (MSAA, SMAA, TAA).

use bevy::anti_alias::smaa::Smaa;
use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::color::palettes::css::SILVER;
use bevy::color::palettes::css::YELLOW;
use bevy::core_pipeline::prepass::MotionVectorPrepass;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::render::camera::MipBias;
use bevy::render::camera::TemporalJitter;
use bevy_lagrange::InputControl;
use bevy_lagrange::LagrangePlugin;
use bevy_lagrange::OrbitCam;
use bevy_lagrange::TrackpadInput;
use bevy_liminal::LiminalPlugin;
use bevy_liminal::Outline;
use bevy_liminal::OutlineCamera;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            LagrangePlugin,
            LiminalPlugin,
        ))
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(
            Update,
            (
                rotate,
                switch_anti_aliasing.run_if(on_message::<KeyboardInput>),
                update_msaa_display,
            ),
        )
        .run();
}

type TaaComponents = (
    TemporalAntiAliasing,
    TemporalJitter,
    MipBias,
    MotionVectorPrepass,
);
type OutlineCameraAaQuery = (
    Entity,
    &'static mut Msaa,
    Option<&'static Smaa>,
    Option<&'static TemporalAntiAliasing>,
);
type OutlineCameraAaState = (
    &'static Msaa,
    Option<&'static Smaa>,
    Option<&'static TemporalAntiAliasing>,
);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 2., 3.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
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

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::from(YELLOW))),
        Transform::from_xyz(0.0, 1.0, 0.0),
        Outline::jump_flood(10.0).build(),
    ));
}

fn rotate(mut query: Query<&mut Transform, With<Outline>>, time: Res<Time>) {
    for mut transform in &mut query {
        let rotation = Quat::from_rotation_y(time.delta_secs() / 6.)
            * Quat::from_rotation_x(time.delta_secs() / 3.0);

        transform.rotation *= rotation;
    }
}

fn switch_anti_aliasing(
    input: Res<ButtonInput<KeyCode>>,
    camera: Single<OutlineCameraAaQuery, With<OutlineCamera>>,
    mut commands: Commands,
) {
    let (camera_entity, mut msaa, smaa, taa) = camera.into_inner();
    let mut camera_commands = commands.entity(camera_entity);

    if input.just_pressed(KeyCode::Digit1) {
        *msaa = Msaa::Off;
    } else if input.just_pressed(KeyCode::Digit2) {
        if taa.is_some() {
            camera_commands.remove::<TaaComponents>();
        }
        *msaa = Msaa::Sample2;
    } else if input.just_pressed(KeyCode::Digit3) {
        if taa.is_some() {
            camera_commands.remove::<TaaComponents>();
        }
        *msaa = Msaa::Sample4;
    } else if input.just_pressed(KeyCode::Digit4) {
        // Note: Sample8 is not supported on all hardware (e.g. Apple Silicon only supports [1, 2,
        // 4])
        if taa.is_some() {
            camera_commands.remove::<TaaComponents>();
        }
        *msaa = Msaa::Sample8;
    }

    if input.just_pressed(KeyCode::KeyS) {
        if smaa.is_some() {
            camera_commands.remove::<Smaa>();
        } else {
            if taa.is_some() {
                camera_commands.remove::<TaaComponents>();
            }
            camera_commands.insert(Smaa::default());
        }
    }

    if input.just_pressed(KeyCode::KeyT) {
        if taa.is_some() {
            camera_commands.remove::<TaaComponents>();
        } else {
            // TAA requires motion vectors and should run with MSAA disabled.
            *msaa = Msaa::Off;
            camera_commands.remove::<Smaa>().insert((
                TemporalAntiAliasing::default(),
                TemporalJitter::default(),
                MipBias::default(),
                MotionVectorPrepass,
            ));
        }
    }
}

#[derive(Component)]
struct MsaaText;

fn setup_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("MSAA:\n1: Off\n2: 2x\n3: 4x (default)\n4: 8x\n\nPost AA:\nS: Toggle SMAA\nT: Toggle TAA"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
        MsaaText,
    ));
}

fn update_msaa_display(
    camera: Single<OutlineCameraAaState, With<OutlineCamera>>,
    mut text_query: Single<&mut Text, With<MsaaText>>,
) {
    let (msaa, smaa, taa) = *camera;
    let current_msaa = match *msaa {
        Msaa::Off => "Off",
        Msaa::Sample2 => "2x",
        Msaa::Sample4 => "4x",
        Msaa::Sample8 => "8x",
    };
    let current_post_aa = if taa.is_some() {
        "TAA"
    } else if smaa.is_some() {
        "SMAA"
    } else {
        "None"
    };
    text_query.0 = format!(
        "MSAA:\n1: Off\n2: 2x\n3: 4x (default)\n4: 8x\n\nPost AA:\nS: Toggle SMAA\nT: Toggle TAA\n\nCurrent MSAA: {current_msaa}\nCurrent Post AA: {current_post_aa}"
    );
}
