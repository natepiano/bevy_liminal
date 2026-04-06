use bevy::core_pipeline::FullscreenShader;
use bevy::prelude::*;
use bevy::render::render_resource::BindGroupLayoutDescriptor;
use bevy::render::render_resource::BindGroupLayoutEntries;
use bevy::render::render_resource::CachedRenderPipelineId;
use bevy::render::render_resource::ColorTargetState;
use bevy::render::render_resource::ColorWrites;
use bevy::render::render_resource::FragmentState;
use bevy::render::render_resource::MultisampleState;
use bevy::render::render_resource::PipelineCache;
use bevy::render::render_resource::PrimitiveState;
use bevy::render::render_resource::RenderPipelineDescriptor;
use bevy::render::render_resource::SamplerBindingType;
use bevy::render::render_resource::ShaderStages;
use bevy::render::render_resource::TextureFormat;
use bevy::render::render_resource::TextureSampleType;
use bevy::render::render_resource::binding_types::sampler;
use bevy::render::render_resource::binding_types::texture_2d;
use bevy::shader::ShaderDefVal;
use bevy_render::render_resource::binding_types::texture_2d_multisampled;
use bevy_render::render_resource::binding_types::texture_depth_2d;

use super::hull_pipeline::DynamicRange;
use super::shaders::COMPOSE_SHADER_HANDLE;

/// Whether the view uses multi-sample anti-aliasing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SampleMode {
    SingleSample,
    MultiSample,
}

impl SampleMode {
    pub(crate) fn from_msaa(msaa: Msaa) -> Self {
        if msaa.samples() > 1 {
            Self::MultiSample
        } else {
            Self::SingleSample
        }
    }
}

/// Identifies one of the four compose pipeline variants by sample mode and dynamic range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ComposeVariant {
    Sdr,
    Hdr,
    MsaaSdr,
    MsaaHdr,
}

impl ComposeVariant {
    pub(crate) const fn new(sample_mode: SampleMode, dynamic_range: DynamicRange) -> Self {
        match (sample_mode, dynamic_range) {
            (SampleMode::SingleSample, DynamicRange::Sdr) => Self::Sdr,
            (SampleMode::SingleSample, DynamicRange::Hdr) => Self::Hdr,
            (SampleMode::MultiSample, DynamicRange::Sdr) => Self::MsaaSdr,
            (SampleMode::MultiSample, DynamicRange::Hdr) => Self::MsaaHdr,
        }
    }

    const fn is_msaa(self) -> bool { matches!(self, Self::MsaaSdr | Self::MsaaHdr) }
}

#[derive(Clone)]
pub(crate) struct ComposePipelines {
    sdr:      CachedRenderPipelineId,
    hdr:      CachedRenderPipelineId,
    msaa_sdr: CachedRenderPipelineId,
    msaa_hdr: CachedRenderPipelineId,
}

impl ComposePipelines {
    pub(crate) const fn get(&self, variant: ComposeVariant) -> CachedRenderPipelineId {
        match variant {
            ComposeVariant::Sdr => self.sdr,
            ComposeVariant::Hdr => self.hdr,
            ComposeVariant::MsaaSdr => self.msaa_sdr,
            ComposeVariant::MsaaHdr => self.msaa_hdr,
        }
    }
}

#[derive(Clone, Resource)]
pub(crate) struct ComposeOutputPipeline {
    pub(crate) layout:      BindGroupLayoutDescriptor,
    pub(crate) msaa_layout: BindGroupLayoutDescriptor,
    pub(crate) pipelines:   ComposePipelines,
}

impl ComposeOutputPipeline {
    pub(crate) const fn pipeline_id(&self, variant: ComposeVariant) -> CachedRenderPipelineId {
        self.pipelines.get(variant)
    }

    pub(crate) const fn layout_for(&self, variant: ComposeVariant) -> &BindGroupLayoutDescriptor {
        if variant.is_msaa() {
            &self.msaa_layout
        } else {
            &self.layout
        }
    }
}

impl FromWorld for ComposeOutputPipeline {
    fn from_world(world: &mut World) -> Self {
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
                targets:     vec![target],
            }),
            primitive:                        PrimitiveState::default(),
            depth_stencil:                    None,
            multisample:                      MultisampleState::default(),
            push_constant_ranges:             vec![],
            zero_initialize_workgroup_memory: false,
        };

        let mut hdr_descriptor = descriptor.clone();
        if let Some(fragment) = hdr_descriptor.fragment.as_mut() {
            fragment.targets = vec![hdr_target.clone()];
        }

        let multisampled_def = ShaderDefVal::Bool("MULTISAMPLED".into(), true);

        let mut msaa_descriptor = descriptor.clone();
        msaa_descriptor.label = Some("outline_compose_output_pipeline_msaa".into());
        msaa_descriptor.layout = vec![msaa_layout.clone()];
        if let Some(fragment) = msaa_descriptor.fragment.as_mut() {
            fragment.shader_defs.push(multisampled_def);
        }

        let mut msaa_hdr_descriptor = msaa_descriptor.clone();
        if let Some(fragment) = msaa_hdr_descriptor.fragment.as_mut() {
            fragment.targets = vec![hdr_target];
        }

        let pipelines = {
            let cache = world.resource_mut::<PipelineCache>();
            ComposePipelines {
                sdr:      cache.queue_render_pipeline(descriptor),
                hdr:      cache.queue_render_pipeline(hdr_descriptor),
                msaa_sdr: cache.queue_render_pipeline(msaa_descriptor),
                msaa_hdr: cache.queue_render_pipeline(msaa_hdr_descriptor),
            }
        };
        Self {
            layout,
            msaa_layout,
            pipelines,
        }
    }
}
