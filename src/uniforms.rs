use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use bytemuck::Pod;
use bytemuck::Zeroable;

use super::extract::ExtractedOutline;
use super::outline::OutlineMethod;

#[derive(Debug, Clone, ShaderType, Pod, Zeroable, Copy)]
#[repr(C)]
pub(crate) struct OutlineUniform {
    pub(crate) intensity:  f32,
    pub(crate) width:      f32,
    pub(crate) priority:   f32,
    pub(crate) overlap:    f32,
    pub(crate) color:      Vec4,
    pub(crate) owner_data: Vec4,
}

impl From<&ExtractedOutline> for OutlineUniform {
    fn from(outline: &ExtractedOutline) -> Self {
        let shell_mode = match outline.mode {
            OutlineMethod::ScreenHull => 1.0,
            _ => 0.0,
        };
        Self {
            intensity:  outline.intensity,
            width:      outline.width,
            priority:   outline.priority,
            overlap:    outline.overlap,
            color:      outline.color,
            owner_data: Vec4::new(outline.owner_id, shell_mode, 0.0, 0.0),
        }
    }
}
