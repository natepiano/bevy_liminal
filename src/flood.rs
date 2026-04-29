use bevy::core_pipeline::FullscreenShader;
use bevy::prelude::*;
use bevy::render::render_resource::BindGroupEntries;
use bevy::render::render_resource::BindGroupLayoutDescriptor;
use bevy::render::render_resource::BindGroupLayoutEntries;
use bevy::render::render_resource::CachedRenderPipelineId;
use bevy::render::render_resource::ColorTargetState;
use bevy::render::render_resource::ColorWrites;
use bevy::render::render_resource::DynamicUniformBuffer;
use bevy::render::render_resource::FilterMode;
use bevy::render::render_resource::FragmentState;
use bevy::render::render_resource::MultisampleState;
use bevy::render::render_resource::Operations;
use bevy::render::render_resource::PipelineCache;
use bevy::render::render_resource::PrimitiveState;
use bevy::render::render_resource::RenderPassColorAttachment;
use bevy::render::render_resource::RenderPassDescriptor;
use bevy::render::render_resource::RenderPipeline;
use bevy::render::render_resource::RenderPipelineDescriptor;
use bevy::render::render_resource::Sampler;
use bevy::render::render_resource::SamplerBindingType;
use bevy::render::render_resource::SamplerDescriptor;
use bevy::render::render_resource::ShaderStages;
use bevy::render::render_resource::ShaderType;
use bevy::render::render_resource::TextureFormat;
use bevy::render::render_resource::TextureSampleType;
use bevy::render::render_resource::binding_types;
use bevy::render::renderer::RenderContext;
use bevy::render::renderer::RenderDevice;
use bevy::render::renderer::RenderQueue;
use bevy::render::texture::CachedTexture;
use bevy_kana::ToUsize;
use bevy_render::render_resource::TextureView;

use super::camera::OutlineCamera;
use super::extract::ExtractedOutlineUniforms;
use super::shaders::FLOOD_SHADER_HANDLE;

#[derive(ShaderType)]
pub(crate) struct JumpFloodUniform {
    pub(crate) step_length: u32,
}

#[derive(Component, Default, Clone)]
pub(crate) struct FloodSettings {
    pub(crate) width: f32,
}

pub(crate) fn prepare_flood_settings(
    mut commands: Commands,
    extracted_outlines: Res<ExtractedOutlineUniforms>,
    cameras: Query<Entity, With<OutlineCamera>>,
) {
    let flood_settings = FloodSettings {
        width: extracted_outlines.max_jump_flood_width,
    };

    for entity in cameras.iter() {
        commands.entity(entity).insert(flood_settings.clone());
    }
}

#[derive(Resource)]
pub(crate) struct JumpFloodPipeline {
    pub(crate) layout:         BindGroupLayoutDescriptor,
    pub(crate) sampler:        Sampler,
    pub(crate) pipeline_id:    CachedRenderPipelineId,
    pub(crate) lookup_buffer:  DynamicUniformBuffer<JumpFloodUniform>,
    pub(crate) lookup_offsets: Vec<u32>,
}

impl FromWorld for JumpFloodPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>().clone();

        let layout = BindGroupLayoutDescriptor::new(
            "outline_jump_flood_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    binding_types::texture_2d(TextureSampleType::Float { filterable: true }), /* flood_texture */
                    binding_types::sampler(SamplerBindingType::Filtering), // texture_sampler
                    binding_types::uniform_buffer::<JumpFloodUniform>(true), // instance
                    binding_types::texture_depth_2d(),                     // depth_texture
                    binding_types::texture_2d(TextureSampleType::Float { filterable: true }), /* appearance_texture */
                ),
            ),
        );
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let fullscreen_shader = world.resource::<FullscreenShader>().clone();

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label:                            Some("outline_jump_flood_pipeline".into()),
                    layout:                           vec![layout.clone()],
                    vertex:                           fullscreen_shader.to_vertex_state(),
                    fragment:                         Some(FragmentState {
                        shader:      FLOOD_SHADER_HANDLE,
                        shader_defs: vec![],
                        entry_point: Some("fragment".into()),
                        targets:     vec![Some(ColorTargetState {
                            format:     TextureFormat::Rgba32Float,
                            blend:      None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive:                        PrimitiveState::default(),
                    depth_stencil:                    None,
                    multisample:                      MultisampleState::default(),
                    push_constant_ranges:             vec![],
                    zero_initialize_workgroup_memory: false,
                });

        let render_queue = world.resource::<RenderQueue>();
        let mut uniform_buffer = DynamicUniformBuffer::new_with_alignment(u64::from(
            render_device.limits().min_uniform_buffer_offset_alignment,
        ));
        let mut offsets = Vec::new();
        for bit in 0..u32::BITS {
            offsets.push(uniform_buffer.push(&JumpFloodUniform {
                step_length: 1 << bit,
            }));
        }
        uniform_buffer.write_buffer(&render_device, render_queue);

        Self {
            layout,
            sampler,
            pipeline_id,
            lookup_buffer: uniform_buffer,
            lookup_offsets: offsets,
        }
    }
}

pub(crate) struct JumpFloodPass<'w> {
    pub(crate) pipeline: &'w JumpFloodPipeline,
    render_pipeline:     &'w RenderPipeline,
    pipeline_cache:      &'w PipelineCache,
}

pub(crate) struct JumpFloodStep<'a> {
    pub(crate) input:              &'a CachedTexture,
    pub(crate) output:             &'a CachedTexture,
    pub(crate) depth_texture:      &'a TextureView,
    pub(crate) appearance_texture: &'a TextureView,
    pub(crate) size:               u32,
}

impl<'w> JumpFloodPass<'w> {
    pub(crate) fn new(world: &'w World) -> Option<Self> {
        let pipeline = world.resource::<JumpFloodPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let render_pipeline = pipeline_cache.get_render_pipeline(pipeline.pipeline_id)?;

        Some(Self {
            pipeline,
            render_pipeline,
            pipeline_cache,
        })
    }

    pub(crate) fn execute(&self, render_context: &mut RenderContext<'_>, step: JumpFloodStep<'_>) {
        let JumpFloodStep {
            input,
            output,
            depth_texture,
            appearance_texture,
            size,
        } = step;
        let Some(lookup_binding) = self.pipeline.lookup_buffer.binding() else {
            return;
        };
        let bind_group = render_context.render_device().create_bind_group(
            "outline_jump_flood_bind_group",
            &self
                .pipeline_cache
                .get_bind_group_layout(&self.pipeline.layout),
            &BindGroupEntries::sequential((
                &input.default_view,
                &self.pipeline.sampler,
                lookup_binding,
                depth_texture,
                appearance_texture,
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label:                    Some("outline_jump_flood_pass"),
            color_attachments:        &[Some(RenderPassColorAttachment {
                view:           &output.default_view,
                resolve_target: None,
                ops:            Operations::default(),
                depth_slice:    None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes:         None,
            occlusion_query_set:      None,
        });

        render_pass.set_render_pipeline(self.render_pipeline);
        render_pass.set_bind_group(
            0,
            &bind_group,
            &[self.pipeline.lookup_offsets[size.to_usize()]],
        );
        render_pass.draw(0..3, 0..1);
    }
}
