use std::ops::Range;

use bevy::asset::UntypedAssetId;
use bevy::prelude::*;
use bevy_render::mesh::allocator::SlabId;
use bevy_render::render_phase::BinnedPhaseItem;
use bevy_render::render_phase::CachedRenderPipelinePhaseItem;
use bevy_render::render_phase::DrawFunctionId;
use bevy_render::render_phase::PhaseItem;
use bevy_render::render_phase::PhaseItemBatchSetKey;
use bevy_render::render_phase::PhaseItemExtraIndex;
use bevy_render::render_resource::CachedRenderPipelineId;
use bevy_render::sync_world::MainEntity;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct OutlineBatchSetKey {
    pub(super) pipeline:      CachedRenderPipelineId,
    pub(super) draw_function: DrawFunctionId,
    pub(super) vertex_slab:   SlabId,
    pub(super) index_slab:    Option<SlabId>,
}

impl PhaseItemBatchSetKey for OutlineBatchSetKey {
    fn indexed(&self) -> bool { self.index_slab.is_some() }
}

/// Including `main_entity` makes each entity its own unique bin. Without it,
/// GPU indirect drawing can reorder entities within a bin, causing
/// `instance_index` to map to the wrong outline uniform and shifting colors
/// between entities. This approach sacrifices multi-entity draw call batching but the
/// single storage buffer and bind group still provide a very large performance win
/// over per-entity buffers.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct OutlineBinKey {
    pub(super) asset_id:    UntypedAssetId,
    pub(super) main_entity: MainEntity,
}

pub(super) struct JfaOutlinePhase {
    pub(super) batch_set_key: OutlineBatchSetKey,
    pub(super) entity:        Entity,
    pub(super) main_entity:   MainEntity,
    pub(super) batch_range:   Range<u32>,
    pub(super) extra_index:   PhaseItemExtraIndex,
}

impl PhaseItem for JfaOutlinePhase {
    #[inline]
    fn entity(&self) -> Entity { self.entity }

    fn main_entity(&self) -> bevy::render::sync_world::MainEntity { self.main_entity }

    fn draw_function(&self) -> bevy::render::render_phase::DrawFunctionId {
        self.batch_set_key.draw_function
    }

    fn batch_range(&self) -> &std::ops::Range<u32> { &self.batch_range }

    fn batch_range_mut(&mut self) -> &mut std::ops::Range<u32> { &mut self.batch_range }

    fn extra_index(&self) -> bevy::render::render_phase::PhaseItemExtraIndex {
        self.extra_index.clone()
    }

    fn batch_range_and_extra_index_mut(
        &mut self,
    ) -> (
        &mut Range<u32>,
        &mut bevy::render::render_phase::PhaseItemExtraIndex,
    ) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl BinnedPhaseItem for JfaOutlinePhase {
    type BinKey = OutlineBinKey;
    type BatchSetKey = OutlineBatchSetKey;

    fn new(
        batch_set_key: Self::BatchSetKey,
        _: Self::BinKey,
        representative_entity: (Entity, MainEntity),
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        Self {
            batch_set_key,
            entity: representative_entity.0,
            main_entity: representative_entity.1,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for JfaOutlinePhase {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId { self.batch_set_key.pipeline }
}

pub(super) struct HullOutlinePhase {
    pub(super) batch_set_key: OutlineBatchSetKey,
    pub(super) entity:        Entity,
    pub(super) main_entity:   MainEntity,
    pub(super) batch_range:   Range<u32>,
    pub(super) extra_index:   PhaseItemExtraIndex,
}

impl PhaseItem for HullOutlinePhase {
    #[inline]
    fn entity(&self) -> Entity { self.entity }

    fn main_entity(&self) -> MainEntity { self.main_entity }

    fn draw_function(&self) -> DrawFunctionId { self.batch_set_key.draw_function }

    fn batch_range(&self) -> &Range<u32> { &self.batch_range }

    fn batch_range_mut(&mut self) -> &mut Range<u32> { &mut self.batch_range }

    fn extra_index(&self) -> PhaseItemExtraIndex { self.extra_index.clone() }

    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl BinnedPhaseItem for HullOutlinePhase {
    type BinKey = OutlineBinKey;
    type BatchSetKey = OutlineBatchSetKey;

    fn new(
        batch_set_key: Self::BatchSetKey,
        _: Self::BinKey,
        representative_entity: (Entity, MainEntity),
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        Self {
            batch_set_key,
            entity: representative_entity.0,
            main_entity: representative_entity.1,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for HullOutlinePhase {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId { self.batch_set_key.pipeline }
}
