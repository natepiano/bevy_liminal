use bevy::prelude::*;
use bevy_liminal::OutlineMethod;

use crate::constants::CUBE_FILL_RATIO_00005;
use crate::constants::CUBE_FILL_RATIO_00010;
use crate::constants::CUBE_FILL_RATIO_00100;
use crate::constants::CUBE_FILL_RATIO_01000;
use crate::constants::CUBE_FILL_RATIO_10000;
use crate::constants::CUBE_FILL_RATIO_50000;
use crate::constants::DEFAULT_OUTLINE_WIDTH;
use crate::grid::GridSpawnSpec;
use crate::grid::spawn_grid;
use crate::state::OutlinePresence;
use crate::viewport::ViewportInfo;

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
