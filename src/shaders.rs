use bevy::asset::load_internal_asset;
use bevy::prelude::*;

use super::constants::COMPOSE_SHADER_HANDLE;
use super::constants::FLOOD_SHADER_HANDLE;
use super::constants::HULL_SHADER_HANDLE;
use super::constants::MASK_SHADER_HANDLE;
use super::constants::VIEW_HELPERS_SHADER_HANDLE;

pub(crate) struct ShaderPlugin;

impl Plugin for ShaderPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            MASK_SHADER_HANDLE,
            "shaders/mask.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            FLOOD_SHADER_HANDLE,
            "shaders/flood.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            COMPOSE_SHADER_HANDLE,
            "shaders/compose_output.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            HULL_SHADER_HANDLE,
            "shaders/hull.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            VIEW_HELPERS_SHADER_HANDLE,
            "shaders/view_helpers.wgsl",
            Shader::from_wgsl
        );
    }
}
