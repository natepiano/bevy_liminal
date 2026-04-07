/// Minimum edge length below which a triangle vertex is considered degenerate
/// and its angle-weighted normal contribution is skipped.
pub(crate) const DEGENERATE_EDGE_THRESHOLD: f32 = 1e-10;

/// Offset added to entity indices when computing owner IDs. Zero is reserved as
/// "no owner" in the shader, so all valid owner IDs start at 1.0.
pub(crate) const OWNER_ID_OFFSET: f32 = 1.0;
