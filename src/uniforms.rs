#![allow(dead_code)]

use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use bytemuck::Pod;
use bytemuck::Zeroable;

use super::ExtractedOutline;
use super::OutlineMode;

#[derive(Debug, Clone, ShaderType, Pod, Zeroable, Copy)]
#[repr(C)]
pub struct OutlineUniform {
    pub intensity:     f32,
    pub width:         f32,
    pub priority:      f32,
    pub overlap:       f32,
    pub outline_color: Vec4,
    pub owner_data:    Vec4,
}

impl From<&ExtractedOutline> for OutlineUniform {
    fn from(outline: &ExtractedOutline) -> Self {
        let shell_mode = match outline.mode {
            OutlineMode::ScreenHull => 1.0,
            _ => 0.0,
        };
        OutlineUniform {
            intensity:     outline.intensity,
            width:         outline.width,
            priority:      outline.priority,
            overlap:       outline.overlap,
            outline_color: outline.color,
            owner_data:    Vec4::new(outline.owner_id, shell_mode, 0.0, 0.0),
        }
    }
}
