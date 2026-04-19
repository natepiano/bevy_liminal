mod outline_api;
mod render_cache;
mod render_data;

pub use outline_api::LineStyle;
pub use outline_api::NoOutline;
pub use outline_api::Outline;
pub use outline_api::OutlineActivity;
pub use outline_api::OutlineCamera;
pub use outline_api::OutlineMethod;
pub use outline_api::OverlapMode;
pub(crate) use render_cache::ActiveOutlineModes;
pub(crate) use render_cache::ExtractedOutlineUniforms;
pub(crate) use render_data::ExtractedOutline;
pub(crate) use render_data::OutlineRenderGraphNode;
pub(crate) use render_data::configure_outline_camera_depth_texture;
