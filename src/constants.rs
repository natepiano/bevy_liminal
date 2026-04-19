use bevy::prelude::LinearRgba;

// Outline rendering constants
/// Multiplicative identity — no scaling applied to the outline color.
pub(crate) const DEFAULT_OUTLINE_INTENSITY: f32 = 1.0;

/// Minimum edge length below which a triangle vertex is considered degenerate
/// and its angle-weighted normal contribution is skipped.
pub(crate) const DEGENERATE_EDGE_THRESHOLD: f32 = 1e-10;

/// Clear color for the JFA seed texture. Negative coordinates signal "no seed"
/// to the flood-fill shader.
pub(crate) const JFA_NO_SEED_CLEAR_COLOR: LinearRgba = LinearRgba::new(-1.0, -1.0, -1.0, 0.0);

/// Shader binding location for the outline normal vertex attribute.
pub(crate) const OUTLINE_NORMAL_SHADER_LOCATION: u32 = 8;

/// Offset added to entity indices when computing owner IDs. Zero is reserved as
/// "no owner" in the shader, so all valid owner IDs start at 1.0.
pub(crate) const OWNER_ID_OFFSET: f32 = 1.0;
