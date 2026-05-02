use bevy::color::palettes::css::YELLOW;
use bevy::prelude::*;
use bevy_kana::ToF32;
use bevy_kana::ToU32;
use bevy_liminal::Outline;
use bevy_liminal::OutlineMethod;
use bevy_liminal::OverlapMode;
use rand::RngExt;

use crate::constants::CAMERA_LOOK_AT;
use crate::constants::CUBE_FILL_RATIO_00005;
use crate::constants::CUBE_FILL_RATIO_00010;
use crate::constants::CUBE_FILL_RATIO_00100;
use crate::constants::CUBE_FILL_RATIO_01000;
use crate::constants::CUBE_FILL_RATIO_10000;
use crate::constants::CUBE_FILL_RATIO_50000;
use crate::constants::DEFAULT_OUTLINE_INTENSITY;
use crate::constants::DEFAULT_OUTLINE_WIDTH;
use crate::constants::DEPTH_SPACING_MULTIPLIER;
use crate::constants::GRID_FILL_FRACTION;
use crate::state::OutlinePresence;

#[derive(Clone, Copy)]
pub(super) struct ScenarioDefinition {
    pub(super) name: &'static str,
    pub(super) key:  KeyCode,
    kind:            ScenarioKind,
}

#[derive(Clone, Copy)]
enum ScenarioKind {
    Grid {
        count:     u32,
        width:     f32,
        cube_fill: f32,
    },
}

pub(super) const SCENARIOS: &[ScenarioDefinition] = &[
    ScenarioDefinition {
        name: "Entities1",
        key:  KeyCode::Digit1,
        kind: ScenarioKind::Grid {
            count:     1,
            width:     DEFAULT_OUTLINE_WIDTH,
            cube_fill: CUBE_FILL_RATIO_00005,
        },
    },
    ScenarioDefinition {
        name: "Entities5",
        key:  KeyCode::Digit2,
        kind: ScenarioKind::Grid {
            count:     5,
            width:     DEFAULT_OUTLINE_WIDTH,
            cube_fill: CUBE_FILL_RATIO_00005,
        },
    },
    ScenarioDefinition {
        name: "Entities10",
        key:  KeyCode::Digit3,
        kind: ScenarioKind::Grid {
            count:     10,
            width:     DEFAULT_OUTLINE_WIDTH,
            cube_fill: CUBE_FILL_RATIO_00010,
        },
    },
    ScenarioDefinition {
        name: "Entities100",
        key:  KeyCode::Digit4,
        kind: ScenarioKind::Grid {
            count:     100,
            width:     DEFAULT_OUTLINE_WIDTH,
            cube_fill: CUBE_FILL_RATIO_00100,
        },
    },
    ScenarioDefinition {
        name: "Entities1000",
        key:  KeyCode::Digit5,
        kind: ScenarioKind::Grid {
            count:     1000,
            width:     DEFAULT_OUTLINE_WIDTH,
            cube_fill: CUBE_FILL_RATIO_01000,
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

#[derive(Component)]
pub(super) struct BenchmarkEntity;

pub(super) struct ViewportInfo {
    right:   Vec3,
    up:      Vec3,
    forward: Vec3,
    center:  Vec3,
    width:   f32,
    height:  f32,
}

pub(super) fn compute_viewport_info(
    camera_transform: &Transform,
    projection: &Projection,
    window: &Window,
) -> ViewportInfo {
    let fov = match projection {
        Projection::Perspective(perspective) => perspective.fov,
        Projection::Orthographic(_) | Projection::Custom(_) => std::f32::consts::FRAC_PI_4,
    };

    let distance = camera_transform.translation.distance(CAMERA_LOOK_AT);
    let aspect = window.width() / window.height();
    let visible_height = 2.0 * distance * (fov / 2.0).tan();
    let visible_width = visible_height * aspect;

    ViewportInfo {
        right:   camera_transform.right().as_vec3(),
        up:      camera_transform.up().as_vec3(),
        forward: camera_transform.forward().as_vec3(),
        center:  CAMERA_LOOK_AT,
        width:   visible_width * GRID_FILL_FRACTION,
        height:  visible_height * GRID_FILL_FRACTION,
    }
}

pub(super) fn spawn_scenario(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    scenario: &ScenarioDefinition,
    viewport: &ViewportInfo,
    outline_presence: OutlinePresence,
    outline_method: OutlineMethod,
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
            outline_presence,
            outline_method,
        },
    );
}

fn random_outline_color() -> Color {
    let mut rng = rand::rng();
    Color::srgb(rng.random(), rng.random(), rng.random())
}

fn build_outline(width: f32, outline_method: OutlineMethod) -> Outline {
    match outline_method {
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
    }
}

fn spawn_grid(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    grid_spawn_spec: GridSpawnSpec<'_>,
) {
    let mesh_handle = meshes.add(Cuboid::default());
    let material_handle = materials.add(Color::from(YELLOW));

    if grid_spawn_spec.count > 100 {
        spawn_3d_grid(commands, &mesh_handle, &material_handle, grid_spawn_spec);
        return;
    }

    let GridSpawnSpec {
        count,
        width,
        cube_fill,
        viewport,
        outline_presence,
        outline_method,
    } = grid_spawn_spec;
    let cols = count.to_f32().sqrt().ceil().to_u32();
    let rows = count.div_ceil(cols);
    let h_spacing = viewport.width / cols.to_f32();
    let v_spacing = viewport.height / rows.to_f32();
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
            if outline_presence == OutlinePresence::Enabled {
                entity.insert(build_outline(width, outline_method));
            }
            spawned += 1;
        }
    }
}

fn spawn_3d_grid(
    commands: &mut Commands,
    mesh_handle: &Handle<Mesh>,
    material_handle: &Handle<StandardMaterial>,
    grid_spawn_spec: GridSpawnSpec<'_>,
) {
    let GridSpawnSpec {
        count,
        width,
        cube_fill,
        viewport,
        outline_presence,
        outline_method,
    } = grid_spawn_spec;
    let cols: u32 = 10;
    let rows: u32 = 10;
    let face_size = cols * rows;
    let layers = count.div_ceil(face_size);
    let h_spacing = viewport.width / cols.to_f32();
    let v_spacing = viewport.height / rows.to_f32();
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
                if outline_presence == OutlinePresence::Enabled {
                    entity.insert(build_outline(width, outline_method));
                }
                spawned += 1;
            }
        }
    }
}

struct GridSpawnSpec<'a> {
    count:            u32,
    width:            f32,
    cube_fill:        f32,
    viewport:         &'a ViewportInfo,
    outline_presence: OutlinePresence,
    outline_method:   OutlineMethod,
}
