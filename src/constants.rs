/// Offset added to entity indices when computing owner IDs. Zero is reserved as
/// "no owner" in the shader, so all valid owner IDs start at 1.0.
pub(crate) const OWNER_ID_OFFSET: f32 = 1.0;
