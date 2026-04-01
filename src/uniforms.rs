use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use bytemuck::Pod;
use bytemuck::Zeroable;

use super::types::ExtractedOutline;
use super::types::OutlineMethod;

#[derive(Debug, Clone, ShaderType, Pod, Zeroable, Copy)]
#[repr(C)]
pub(super) struct OutlineUniform {
    pub(super) intensity:     f32,
    pub(super) width:         f32,
    pub(super) priority:      f32,
    pub(super) overlap:       f32,
    pub(super) outline_color: Vec4,
    pub(super) owner_data:    Vec4,
}

impl From<&ExtractedOutline> for OutlineUniform {
    fn from(outline: &ExtractedOutline) -> Self {
        let shell_mode = match outline.mode {
            OutlineMethod::ScreenHull => 1.0,
            _ => 0.0,
        };
        Self {
            intensity:     outline.intensity,
            width:         outline.width,
            priority:      outline.priority,
            overlap:       outline.overlap,
            outline_color: outline.color,
            owner_data:    Vec4::new(outline.owner_id, shell_mode, 0.0, 0.0),
        }
    }
}
