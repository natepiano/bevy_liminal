use bevy::core_pipeline::prepass::MotionVectorPrepass;
use bevy::core_pipeline::prepass::NormalPrepass;
use bevy::ecs::change_detection::Tick;
use bevy::pbr::MeshPipelineKey;
use bevy::pbr::RenderMeshInstances;
use bevy::prelude::*;
use bevy_render::batching::gpu_preprocessing::GpuPreprocessingSupport;
use bevy_render::mesh::RenderMesh;
use bevy_render::mesh::allocator::MeshAllocator;
use bevy_render::render_asset::RenderAssets;
use bevy_render::render_phase::BinnedRenderPhaseType;
use bevy_render::render_phase::DrawFunctions;
use bevy_render::render_phase::ViewBinnedRenderPhases;
use bevy_render::render_resource::PipelineCache;
use bevy_render::render_resource::SpecializedMeshPipelines;
use bevy_render::view::ExtractedView;
use bevy_render::view::RenderVisibleEntities;

use super::ActiveOutlineModes;
use super::DrawHull;
use super::DrawOutline;
use super::ExtractedOutlineUniforms;
use super::HullOutlinePhase;
use super::JfaOutlinePhase;
use super::OutlineCamera;
use super::OutlineMethod;
use super::hull_pipeline::HullPipeline;
use super::hull_pipeline::HullPipelineKey;
use super::mask_pipeline::MaskPipelineKey;
use super::mask_pipeline::MeshMaskPipeline;
use crate::mask::OutlineBatchSetKey;
use crate::mask::OutlineBinKey;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn queue_outline(
    extracted_outlines: Res<ExtractedOutlineUniforms>,
    draw_functions: Res<DrawFunctions<JfaOutlinePhase>>,
    mut outline_phases: ResMut<ViewBinnedRenderPhases<JfaOutlinePhase>>,
    mesh_outline_pipeline: Res<MeshMaskPipeline>,
    mut mesh_outline_pipelines: ResMut<SpecializedMeshPipelines<MeshMaskPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    mesh_allocator: Res<MeshAllocator>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    active: Res<ActiveOutlineModes>,
    views: Query<
        (
            Entity,
            &ExtractedView,
            &RenderVisibleEntities,
            &Msaa,
            Has<NormalPrepass>,
            Has<MotionVectorPrepass>,
        ),
        With<OutlineCamera>,
    >,
    mut change_tick: Local<Tick>,
) {
    let draw_function = draw_functions.read().id::<DrawOutline>();

    for (
        _view_entity,
        view,
        visible_entities,
        msaa,
        has_normal_prepass,
        has_motion_vector_prepass,
    ) in views.iter()
    {
        let Some(outline_phase) = outline_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        let mut view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::DEPTH_PREPASS
            | MeshPipelineKey::from_hdr(view.hdr);

        if has_normal_prepass {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }
        if has_motion_vector_prepass {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }

        for &(render_entity, main_entity) in visible_entities.get::<Mesh3d>().iter() {
            if !extracted_outlines.by_main_entity.contains_key(&main_entity) {
                continue;
            }
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(main_entity)
            else {
                tracing::warn!(target: "bevy_liminal", "No mesh instance found for entity {:?}", main_entity);
                continue;
            };

            let (vertex_slab, index_slab) = mesh_allocator.mesh_slabs(&mesh_instance.mesh_asset_id);

            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                tracing::warn!(target: "bevy_liminal", "No mesh found for entity {:?}", main_entity);
                continue;
            };

            let mut mesh_key = view_key;
            mesh_key |= MeshPipelineKey::from_primitive_topology(mesh.primitive_topology())
                | MeshPipelineKey::from_bits_retain(mesh.key_bits.bits());

            let Ok(pipeline_id) = mesh_outline_pipelines.specialize(
                &pipeline_cache,
                &mesh_outline_pipeline,
                MaskPipelineKey {
                    mesh_key,
                    has_hull: active.has_hull,
                },
                &mesh.layout,
            ) else {
                tracing::warn!(target: "bevy_liminal", "Failed to specialize mesh pipeline");
                continue;
            };

            let next_change_tick = change_tick.get() + 1;
            change_tick.set(next_change_tick);

            outline_phase.add(
                OutlineBatchSetKey {
                    pipeline: pipeline_id,
                    draw_function,
                    vertex_slab: vertex_slab.unwrap_or_default(),
                    index_slab,
                },
                OutlineBinKey {
                    asset_id: mesh_instance.mesh_asset_id.untyped(),
                    main_entity,
                },
                (render_entity, main_entity),
                mesh_instance.current_uniform_index,
                BinnedRenderPhaseType::mesh(
                    mesh_instance.should_batch(),
                    &gpu_preprocessing_support,
                ),
                *change_tick,
            );
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn queue_hull_outline(
    active: Res<ActiveOutlineModes>,
    extracted_outlines: Res<ExtractedOutlineUniforms>,
    draw_functions: Res<DrawFunctions<HullOutlinePhase>>,
    mut outline_phases: ResMut<ViewBinnedRenderPhases<HullOutlinePhase>>,
    hull_pipeline: Res<HullPipeline>,
    mut hull_pipelines: ResMut<SpecializedMeshPipelines<HullPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    mesh_allocator: Res<MeshAllocator>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    views: Query<
        (
            Entity,
            &ExtractedView,
            &RenderVisibleEntities,
            &Msaa,
            Has<NormalPrepass>,
        ),
        With<OutlineCamera>,
    >,
    mut change_tick: Local<Tick>,
) {
    if !active.has_hull {
        return;
    }

    let draw_function = draw_functions.read().id::<DrawHull>();

    for (_view_entity, view, visible_entities, msaa, has_normal_prepass) in views.iter() {
        let Some(outline_phase) = outline_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        let mut view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::DEPTH_PREPASS
            | MeshPipelineKey::from_hdr(view.hdr);

        if has_normal_prepass {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }

        for &(render_entity, main_entity) in visible_entities.get::<Mesh3d>().iter() {
            let Some(outline) = extracted_outlines.by_main_entity.get(&main_entity) else {
                continue;
            };
            if outline.mode != OutlineMethod::WorldHull && outline.mode != OutlineMethod::ScreenHull
            {
                continue;
            }

            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(main_entity)
            else {
                tracing::warn!(target: "bevy_liminal", "No mesh instance found for entity {:?}", main_entity);
                continue;
            };

            let (vertex_slab, index_slab) = mesh_allocator.mesh_slabs(&mesh_instance.mesh_asset_id);

            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                tracing::warn!(target: "bevy_liminal", "No mesh found for entity {:?}", main_entity);
                continue;
            };

            let mut mesh_key = view_key;
            mesh_key |= MeshPipelineKey::from_primitive_topology(mesh.primitive_topology())
                | MeshPipelineKey::from_bits_retain(mesh.key_bits.bits());

            let Ok(pipeline_id) = hull_pipelines.specialize(
                &pipeline_cache,
                &hull_pipeline,
                HullPipelineKey {
                    mesh_key,
                    hdr: view.hdr,
                },
                &mesh.layout,
            ) else {
                tracing::warn!(target: "bevy_liminal", "Failed to specialize hull mesh pipeline");
                continue;
            };

            let next_change_tick = change_tick.get() + 1;
            change_tick.set(next_change_tick);

            outline_phase.add(
                OutlineBatchSetKey {
                    pipeline: pipeline_id,
                    draw_function,
                    vertex_slab: vertex_slab.unwrap_or_default(),
                    index_slab,
                },
                OutlineBinKey {
                    asset_id: mesh_instance.mesh_asset_id.untyped(),
                    main_entity,
                },
                (render_entity, main_entity),
                mesh_instance.current_uniform_index,
                BinnedRenderPhaseType::mesh(
                    mesh_instance.should_batch(),
                    &gpu_preprocessing_support,
                ),
                *change_tick,
            );
        }
    }
}
