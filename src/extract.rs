use bevy::prelude::*;
use bevy_render::Extract;
use bevy_render::sync_world::MainEntity;

use super::types::ActiveOutlineModes;
use super::types::ExtractedOutline;
use super::types::ExtractedOutlineUniforms;
use super::types::Outline;

type OutlineEntityAndOutline = (Entity, &'static Outline);
type AddedOrChangedOutlineFilter = (With<Mesh3d>, Or<(Added<Outline>, Changed<Outline>)>);
type AddedOutlineFilter = (Added<Mesh3d>, With<Outline>);

pub(crate) fn extract_outline_uniforms(
    mut extracted_outlines: ResMut<ExtractedOutlineUniforms>,
    added_or_changed_outlines: Extract<Query<OutlineEntityAndOutline, AddedOrChangedOutlineFilter>>,
    added_mesh_outlines: Extract<Query<OutlineEntityAndOutline, AddedOutlineFilter>>,
    mut removed_outlines: Extract<RemovedComponents<Outline>>,
    mut removed_meshes: Extract<RemovedComponents<Mesh3d>>,
) {
    let mut dirty = false;

    for entity in removed_outlines.read() {
        dirty |= extracted_outlines
            .by_main_entity
            .remove(&MainEntity::from(entity))
            .is_some();
    }

    for entity in removed_meshes.read() {
        dirty |= extracted_outlines
            .by_main_entity
            .remove(&MainEntity::from(entity))
            .is_some();
    }

    for (entity, outline) in &added_or_changed_outlines {
        if outline.activity.is_enabled() {
            dirty |= extracted_outlines.upsert(
                MainEntity::from(entity),
                ExtractedOutline::from_main_world(entity, outline),
            );
        } else {
            dirty |= extracted_outlines
                .by_main_entity
                .remove(&MainEntity::from(entity))
                .is_some();
        }
    }

    for (entity, outline) in &added_mesh_outlines {
        if outline.activity.is_enabled() {
            dirty |= extracted_outlines.upsert(
                MainEntity::from(entity),
                ExtractedOutline::from_main_world(entity, outline),
            );
        }
    }

    if dirty {
        extracted_outlines.recompute_flags_and_width();
    }
}

pub(crate) fn update_active_outline_modes(
    extracted_outlines: Res<ExtractedOutlineUniforms>,
    mut active: ResMut<ActiveOutlineModes>,
) {
    active.methods = extracted_outlines.methods;
}
