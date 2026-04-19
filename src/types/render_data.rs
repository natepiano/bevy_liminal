use bevy::prelude::*;
use bevy::render::render_resource::TextureUsages;
use bevy_kana::ToF32;
use bevy_render::render_graph::RenderLabel;

use super::outline_api::Outline;
use super::outline_api::OutlineCamera;
use super::outline_api::OutlineMethod;
use super::outline_api::OverlapMode;
use crate::constants::OWNER_ID_OFFSET;

/// GPU-ready outline data extracted from the main world.
#[derive(Debug, Reflect, Clone, PartialEq)]
pub(crate) struct ExtractedOutline {
    /// Color multiplier for HDR glow via bloom.
    pub(crate) intensity: f32,
    /// Outline width in pixels or world units depending on `mode`.
    pub(crate) width:     f32,
    /// Draw priority for ordering (reserved for future use).
    pub(crate) priority:  f32,
    /// Shader overlap factor derived from `OverlapMode`.
    pub(crate) overlap:   f32,
    /// Unique owner ID used for per-mesh and grouped overlap resolution.
    pub(crate) owner_id:  f32,
    /// Linear RGBA outline color as a `Vec4`.
    pub(crate) color:     Vec4,
    /// Which outline algorithm this entity uses.
    pub(crate) mode:      OutlineMethod,
}

impl ExtractedOutline {
    pub(crate) fn from_main_world(entity: Entity, outline: &Outline) -> Self {
        let linear_color: LinearRgba = outline.color.into();
        let owner_entity = match outline.overlap {
            OverlapMode::Grouped => outline.group_source.unwrap_or(entity),
            _ => entity,
        };
        Self {
            intensity: outline.intensity,
            width:     outline.width,
            priority:  0.0,
            overlap:   outline.overlap.as_shader_factor(),
            owner_id:  owner_entity.index().index().to_f32() + OWNER_ID_OFFSET,
            color:     linear_color.to_vec4(),
            mode:      outline.mode,
        }
    }
}

/// Render graph label for the outline pass.
#[derive(Copy, Clone, Debug, RenderLabel, Hash, PartialEq, Eq)]
pub(crate) enum OutlineRenderGraphNode {
    /// The main outline render node that runs mask, flood, hull, and compose sub-passes.
    OutlineNode,
}

/// Ensures the main pass depth texture has `TEXTURE_BINDING` so the compose shader
/// can sample it for correct occlusion of transmissive/transparent geometry.
///
/// Fires once when `OutlineCamera` is added, rather than polling every frame.
///
/// Needs to run in the main app because `Camera3d::depth_texture_usages` controls
/// how the GPU texture is allocated — by the time extraction runs, it's too late.
///
/// See `bevy_pbr::atmosphere::configure_camera_depth_usages` for the same pattern in Bevy.
pub(crate) fn configure_outline_camera_depth_texture(
    added: On<Add, OutlineCamera>,
    mut cameras: Query<&mut Camera3d>,
) {
    if let Ok(mut camera_3d) = cameras.get_mut(added.entity) {
        let mut usages = TextureUsages::from(camera_3d.depth_texture_usages);
        usages |= TextureUsages::TEXTURE_BINDING;
        camera_3d.depth_texture_usages = usages.into();
    }
}
