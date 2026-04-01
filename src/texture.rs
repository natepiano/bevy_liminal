use bevy::core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT;
use bevy::prelude::*;
use bevy_render::camera::ExtractedCamera;
use bevy_render::render_resource::Texture;
use bevy_render::render_resource::TextureDescriptor;
use bevy_render::renderer::RenderDevice;
use bevy_render::texture::CachedTexture;
use bevy_render::texture::TextureCache;
use wgpu_types::Extent3d;
use wgpu_types::TextureDimension;
use wgpu_types::TextureFormat;
use wgpu_types::TextureUsages;

use super::types::ActiveOutlineModes;
use super::types::OutlineCamera;

#[derive(Clone, Component)]
pub(super) struct FloodTextures {
    pub(super) flip:                  bool,
    // Textures for storing input-output of flood passes
    pub(super) input:                 CachedTexture,
    pub(super) output:                CachedTexture,
    /// A dedicated depth texture for mesh outlines to later compare against
    /// global depth
    pub(super) outline_depth_texture: Texture,
    /// Stores outline color and mesh data
    pub(super) appearance_texture:    CachedTexture,
    /// Stores per-mesh owner ID in x channel — only allocated when hull outlines are active
    pub(super) owner_texture:         Option<CachedTexture>,
}

impl FloodTextures {
    pub(super) const fn input(&self) -> &CachedTexture {
        if self.flip { &self.output } else { &self.input }
    }

    pub(super) const fn output(&self) -> &CachedTexture {
        if self.flip { &self.input } else { &self.output }
    }

    pub(super) const fn flip(&mut self) { self.flip = !self.flip; }
}

pub(super) fn prepare_flood_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    active: Res<ActiveOutlineModes>,
    cameras: Query<(Entity, &ExtractedCamera), With<OutlineCamera>>,
) {
    for (entity, camera) in cameras.iter() {
        let Some(target_size) = camera.physical_target_size else {
            continue;
        };

        let size = Extent3d {
            width:                 target_size.x,
            height:                target_size.y,
            depth_or_array_layers: 1,
        };

        let texture_descriptor = TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };

        // Create the depth texture
        let depth_texture = render_device.create_texture(&TextureDescriptor {
            label: Some("outline depth texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: CORE_3D_DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT  // For using as depth buffer
        | TextureUsages::TEXTURE_BINDING, // For sampling in composite pass
            view_formats: &[],
        });

        let owner_texture = if active.has_hull {
            Some(texture_cache.get(&render_device, texture_descriptor.clone()))
        } else {
            None
        };

        commands.entity(entity).insert(FloodTextures {
            flip: false,
            input: texture_cache.get(&render_device, texture_descriptor.clone()),
            output: texture_cache.get(&render_device, texture_descriptor.clone()),
            outline_depth_texture: depth_texture,
            appearance_texture: texture_cache.get(&render_device, texture_descriptor),
            owner_texture,
        });
        texture_cache.update();
    }
}
