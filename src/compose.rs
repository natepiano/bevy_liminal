use bevy::core_pipeline::FullscreenShader;
use bevy::prelude::*;
use bevy::render::render_resource::BindGroupLayoutDescriptor;
use bevy::render::render_resource::BindGroupLayoutEntries;
use bevy::render::render_resource::CachedRenderPipelineId;
use bevy::render::render_resource::FragmentState;
use bevy::render::render_resource::PipelineCache;
use bevy::render::render_resource::RenderPipelineDescriptor;
use bevy::render::render_resource::binding_types::sampler;
use bevy::render::render_resource::binding_types::texture_2d;
use bevy::render::renderer::RenderDevice;
use bevy::shader::ShaderDefVal;
use bevy_render::render_resource::binding_types::texture_2d_multisampled;
use bevy_render::render_resource::binding_types::texture_depth_2d;
use wgpu_types::ColorTargetState;
use wgpu_types::ColorWrites;
use wgpu_types::MultisampleState;
use wgpu_types::PrimitiveState;
use wgpu_types::SamplerBindingType;
use wgpu_types::ShaderStages;
use wgpu_types::TextureFormat;
use wgpu_types::TextureSampleType;

use crate::shaders::COMPOSE_SHADER_HANDLE;

#[derive(Clone, Resource)]
pub struct ComposeOutputPipeline {
    pub layout:               BindGroupLayoutDescriptor,
    pub msaa_layout:          BindGroupLayoutDescriptor,
    pub pipeline_id:          CachedRenderPipelineId,
    pub hdr_pipeline_id:      CachedRenderPipelineId,
    pub msaa_pipeline_id:     CachedRenderPipelineId,
    pub msaa_hdr_pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for ComposeOutputPipeline {
    fn from_world(world: &mut World) -> Self {
        let _render_device = world.resource::<RenderDevice>();

        let layout = BindGroupLayoutDescriptor::new(
            "outline_compose_output_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_depth_2d(),
                    texture_depth_2d(),
                    texture_depth_2d(),
                ),
            ),
        );

        let msaa_layout = BindGroupLayoutDescriptor::new(
            "outline_compose_output_bind_group_layout_msaa",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d_multisampled(TextureSampleType::Depth),
                    texture_depth_2d(),
                    texture_2d_multisampled(TextureSampleType::Depth),
                ),
            ),
        );

        let target = Some(ColorTargetState {
            format:     TextureFormat::bevy_default(),
            blend:      None,
            write_mask: ColorWrites::ALL,
        });
        let hdr_target = Some(ColorTargetState {
            format:     TextureFormat::Rgba16Float,
            blend:      None,
            write_mask: ColorWrites::ALL,
        });

        let descriptor = RenderPipelineDescriptor {
            label:                            Some("outline_compose_output_pipeline".into()),
            layout:                           vec![layout.clone()],
            vertex:                           world
                .resource::<FullscreenShader>()
                .clone()
                .to_vertex_state(),
            fragment:                         Some(FragmentState {
                shader:      COMPOSE_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Some("fragment".into()),
                targets:     vec![target.clone()],
            }),
            primitive:                        PrimitiveState::default(),
            depth_stencil:                    None,
            multisample:                      MultisampleState::default(),
            push_constant_ranges:             vec![],
            zero_initialize_workgroup_memory: false,
        };

        let mut hdr_descriptor = descriptor.clone();
        hdr_descriptor.fragment.as_mut().unwrap().targets = vec![hdr_target.clone()];

        let multisampled_def = ShaderDefVal::Bool("MULTISAMPLED".into(), true);

        let mut msaa_descriptor = descriptor.clone();
        msaa_descriptor.label = Some("outline_compose_output_pipeline_msaa".into());
        msaa_descriptor.layout = vec![msaa_layout.clone()];
        msaa_descriptor
            .fragment
            .as_mut()
            .unwrap()
            .shader_defs
            .push(multisampled_def.clone());

        let mut msaa_hdr_descriptor = msaa_descriptor.clone();
        msaa_hdr_descriptor.fragment.as_mut().unwrap().targets = vec![hdr_target];

        let (pipeline_id, hdr_pipeline_id, msaa_pipeline_id, msaa_hdr_pipeline_id) = {
            let cache = world.resource_mut::<PipelineCache>();
            (
                cache.queue_render_pipeline(descriptor),
                cache.queue_render_pipeline(hdr_descriptor),
                cache.queue_render_pipeline(msaa_descriptor),
                cache.queue_render_pipeline(msaa_hdr_descriptor),
            )
        };
        Self {
            layout,
            msaa_layout,
            pipeline_id,
            hdr_pipeline_id,
            msaa_pipeline_id,
            msaa_hdr_pipeline_id,
        }
    }
}
