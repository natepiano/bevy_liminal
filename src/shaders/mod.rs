use bevy::asset::Handle;
use bevy::asset::load_internal_asset;
use bevy::asset::uuid_handle;
use bevy::prelude::*;

pub(crate) const MASK_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("4c41a7eb-b802-4e76-97f1-3327d80743dd");

pub(crate) const FLOOD_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("a06a9919-18e3-4e91-a312-a1463bb6d719");

pub(crate) const COMPOSE_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("6fe0f3ef-e31f-40e7-a20a-ed002ac4bb3f");
pub(crate) const HULL_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("6b6c1df4-e857-4f9f-a4a3-4ca5f0bc4df4");

pub(crate) const VIEW_HELPERS_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("a3e7c2b1-9d4f-4e8a-b5c6-1f2d3e4a5b6c");

pub(crate) struct ShaderPlugin;

impl Plugin for ShaderPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, MASK_SHADER_HANDLE, "mask.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, FLOOD_SHADER_HANDLE, "flood.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            COMPOSE_SHADER_HANDLE,
            "compose_output.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, HULL_SHADER_HANDLE, "hull.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            VIEW_HELPERS_SHADER_HANDLE,
            "view_helpers.wgsl",
            Shader::from_wgsl
        );
    }
}
