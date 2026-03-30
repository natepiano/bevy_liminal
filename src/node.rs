use bevy::core_pipeline::prepass::ViewPrepassTextures;
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy_kana::ToU32;
use bevy_render::camera::ExtractedCamera;
use bevy_render::render_graph::NodeRunError;
use bevy_render::render_graph::RenderGraphContext;
use bevy_render::render_graph::ViewNode;
use bevy_render::render_phase::ViewBinnedRenderPhases;
use bevy_render::render_resource::BindGroupEntries;
use bevy_render::render_resource::LoadOp;
use bevy_render::render_resource::Operations;
use bevy_render::render_resource::PipelineCache;
use bevy_render::render_resource::RenderPassColorAttachment;
use bevy_render::render_resource::RenderPassDepthStencilAttachment;
use bevy_render::render_resource::RenderPassDescriptor;
use bevy_render::render_resource::StoreOp;
use bevy_render::render_resource::TextureViewDescriptor;
use bevy_render::renderer::RenderContext;
use bevy_render::view::ExtractedView;
use bevy_render::view::ViewDepthTexture;
use bevy_render::view::ViewTarget;

use super::compose::ComposeOutputPipeline;
use super::flood::FloodSettings;
use super::flood::JumpFloodPass;
use super::texture::FloodTextures;
use crate::HullOutlinePhase;
use crate::JfaOutlinePhase;

#[derive(Default)]
pub struct OutlineNode;
#[allow(clippy::too_many_lines)]
impl ViewNode for OutlineNode {
    type ViewQuery = (
        Entity,
        &'static ExtractedView,
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static FloodTextures,
        &'static ViewPrepassTextures,
        &'static ViewDepthTexture,
        &'static Msaa,
        &'static FloodSettings,
    );

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            view_entity,
            extracted_view,
            camera,
            view_target,
            flood_textures,
            prepass_textures,
            view_depth_texture,
            msaa,
            flood_settings,
        ): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(outline_phases) = world.get_resource::<ViewBinnedRenderPhases<JfaOutlinePhase>>()
        else {
            return Ok(());
        };
        let outline_phase = outline_phases.get(&extracted_view.retained_view_entity);
        let hull_phase = world
            .get_resource::<ViewBinnedRenderPhases<HullOutlinePhase>>()
            .and_then(|phases| phases.get(&extracted_view.retained_view_entity));

        let has_jfa = outline_phase.is_some_and(|phase| !phase.is_empty());
        let has_hull = hull_phase.is_some_and(|phase| !phase.is_empty());
        if !has_jfa && !has_hull {
            return Ok(());
        }

        let outline_phase = outline_phase.filter(|phase| !phase.is_empty());
        let hull_phase = hull_phase.filter(|phase| !phase.is_empty());

        let Some(jump_flood_pass) = JumpFloodPass::new(world) else {
            return Ok(());
        };
        let mut flood_textures = flood_textures.clone();
        let Some(global_depth) = prepass_textures.depth.as_ref() else {
            tracing::warn!("No global depth texture found");
            return Ok(());
        };

        // Note: Textures are cleared via LoadOp::Clear in the render passes below
        // clear_texture is not supported in WebGPU backend

        let flood_color_attachment = RenderPassColorAttachment {
            view:           &flood_textures.output.default_view,
            resolve_target: None,
            ops:            Operations {
                load:  LoadOp::Clear(wgpu_types::Color {
                    r: -1.0,
                    g: -1.0,
                    b: -1.0,
                    a: 0.0,
                }),
                store: StoreOp::Store,
            },
            depth_slice:    None,
        };

        let appearance_color_attachment = RenderPassColorAttachment {
            view:           &flood_textures.appearance_texture.default_view,
            resolve_target: None,
            ops:            Operations {
                load:  LoadOp::Clear(wgpu_types::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                }),
                store: StoreOp::Store,
            },
            depth_slice:    None,
        };
        let owner_color_attachment =
            flood_textures
                .owner_texture
                .as_ref()
                .map(|tex| RenderPassColorAttachment {
                    view:           &tex.default_view,
                    resolve_target: None,
                    ops:            Operations {
                        load:  LoadOp::Clear(wgpu_types::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: StoreOp::Store,
                    },
                    depth_slice:    None,
                });

        let outline_depth_view = flood_textures
            .outline_depth_texture
            .create_view(&TextureViewDescriptor::default());

        let mut color_attachments: Vec<Option<RenderPassColorAttachment>> = vec![
            Some(flood_color_attachment),
            Some(appearance_color_attachment),
        ];
        if let Some(attachment) = owner_color_attachment {
            color_attachments.push(Some(attachment));
        }

        {
            let mut init_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label:                    Some("outline_flood_init"),
                color_attachments:        &color_attachments,
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view:        &outline_depth_view,
                    depth_ops:   Some(Operations {
                        load:  LoadOp::Clear(0.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes:         None,
                occlusion_query_set:      None,
            });

            if let Some(viewport) = camera.viewport.as_ref() {
                init_pass.set_camera_viewport(viewport);
            }

            if let Some(outline_phase) = outline_phase
                && let Err(err) = outline_phase.render(&mut init_pass, world, view_entity)
            {
                error!("Error encountered while rendering the outline flood init phase {err:?}");
            }
        }

        if let Some(hull_phase) = hull_phase {
            let mut hull_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label:                    Some("hull_outline_pass"),
                color_attachments:        &[Some(view_target.get_color_attachment())],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view:        view_depth_texture.view(),
                    depth_ops:   Some(Operations {
                        load:  LoadOp::Load,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes:         None,
                occlusion_query_set:      None,
            });

            if let Some(viewport) = camera.viewport.as_ref() {
                hull_pass.set_camera_viewport(viewport);
            }

            if let Err(err) = hull_phase.render(&mut hull_pass, world, view_entity) {
                error!("Error encountered while rendering hull outline phase {err:?}");
            }
        }

        let Some(active) = world.get_resource::<crate::ActiveOutlineModes>() else {
            return Ok(());
        };
        if !active.has_jfa {
            return Ok(());
        }

        let Some(compose_pipeline) = world.get_resource::<ComposeOutputPipeline>() else {
            // Skip
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();

        let is_msaa = msaa.samples() > 1;
        let pipeline_id = match (is_msaa, view_target.is_hdr()) {
            (true, true) => compose_pipeline.msaa_hdr_pipeline_id,
            (true, false) => compose_pipeline.msaa_pipeline_id,
            (false, true) => compose_pipeline.hdr_pipeline_id,
            (false, false) => compose_pipeline.pipeline_id,
        };

        // Get the pipeline from the cache
        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id) else {
            // Skip
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        // Flooding!

        let outline_width: f32 = flood_settings.width;

        let passes = if outline_width > 0.0 {
            ((outline_width * 2.0).ceil().to_u32() / 2 + 1)
                .next_power_of_two()
                .trailing_zeros()
                + 1
        } else {
            0
        };

        for size in (0..passes).rev() {
            flood_textures.flip();
            jump_flood_pass.execute(
                render_context,
                flood_textures.input(),
                flood_textures.output(),
                &outline_depth_view,
                &flood_textures.appearance_texture.default_view,
                size,
            );
        }

        let layout = if is_msaa {
            &compose_pipeline.msaa_layout
        } else {
            &compose_pipeline.layout
        };
        let bind_group = render_context.render_device().create_bind_group(
            "compose_output_bind_group",
            &pipeline_cache.get_bind_group_layout(layout),
            &BindGroupEntries::sequential((
                // binding 0: screen_texture - The original scene color
                post_process.source,
                // binding 1: texture_sampler - Use the sampler created for the pipeline
                &jump_flood_pass.pipeline.sampler,
                // binding 2: flood_texture - The flood output texture
                &flood_textures.output.default_view,
                // binding 3: appearance_texture - The appearance data texture
                &flood_textures.appearance_texture.default_view,
                // binding 4: depth_texture - Prepass depth texture
                &global_depth.texture.default_view,
                // binding 5: outline_depth_texture - Use the outline depth texture
                &outline_depth_view,
                // binding 6: main_depth_texture - Main pass depth (includes transmissive geometry)
                view_depth_texture.view(),
            )),
        );

        // Composite pass — write directly to `post_process.destination` rather than using
        // `view_target.get_color_attachment()` because the latter returns the multisampled
        // main texture when MSAA is enabled, which would require the pipeline to match
        // the MSAA sample count. The post-process destination is always single-sample.
        {
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label:                    Some("post_process_pass"),
                color_attachments:        &[Some(RenderPassColorAttachment {
                    view:           post_process.destination,
                    resolve_target: None,
                    ops:            Operations::default(),
                    depth_slice:    None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes:         None,
                occlusion_query_set:      None,
            });

            render_pass.set_render_pipeline(pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
        Ok(())
    }
}
