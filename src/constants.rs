use bevy::asset::Handle;
use bevy::asset::uuid_handle;
use bevy::mesh::MeshVertexAttribute;
use bevy::prelude::LinearRgba;
use bevy::prelude::Shader;
use bevy::render::render_resource::VertexFormat;

// outline rendering constants
/// Custom vertex attribute storing pre-computed smoothed outline normals.
///
/// These normals are averaged across all faces sharing a vertex position,
/// weighted by the angle at each face, producing smooth silhouette extrusion
/// even on hard-edged meshes.
pub const ATTRIBUTE_OUTLINE_NORMAL: MeshVertexAttribute =
    MeshVertexAttribute::new("Outline_Normal", 988_540_917, VertexFormat::Float32x3);

/// Multiplicative identity — no scaling applied to the outline color.
pub(crate) const DEFAULT_OUTLINE_INTENSITY: f32 = 1.0;

/// Minimum edge length below which a triangle vertex is considered degenerate
/// and its angle-weighted normal contribution is skipped.
pub(crate) const DEGENERATE_EDGE_THRESHOLD: f32 = 1e-10;

/// Clear color for the JFA seed texture. Negative coordinates signal "no seed"
/// to the flood-fill shader.
pub(crate) const JFA_NO_SEED_CLEAR_COLOR: LinearRgba = LinearRgba::new(-1.0, -1.0, -1.0, 0.0);

/// Reverse-Z far-plane sentinel used when clearing the outline depth texture.
/// Cleared to 0.0 so that any rendered outline fragment (closer than the far
/// plane) will pass the depth comparison.
pub(crate) const OUTLINE_DEPTH_FAR_PLANE_CLEAR: f32 = 0.0;

/// Shader binding location for the outline normal vertex attribute.
pub(crate) const OUTLINE_NORMAL_SHADER_LOCATION: u32 = 8;

/// Offset added to entity indices when computing owner IDs. Zero is reserved as
/// "no owner" in the shader, so all valid owner IDs start at 1.0.
pub(crate) const OWNER_ID_OFFSET: f32 = 1.0;

// shader handles
pub(crate) const COMPOSE_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("6fe0f3ef-e31f-40e7-a20a-ed002ac4bb3f");
pub(crate) const FLOOD_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("a06a9919-18e3-4e91-a312-a1463bb6d719");
pub(crate) const HULL_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("6b6c1df4-e857-4f9f-a4a3-4ca5f0bc4df4");
pub(crate) const MASK_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("4c41a7eb-b802-4e76-97f1-3327d80743dd");
pub(crate) const VIEW_HELPERS_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("a3e7c2b1-9d4f-4e8a-b5c6-1f2d3e4a5b6c");
