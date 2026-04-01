use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::prelude::*;
use bevy::render::render_resource::TextureUsages;
use bevy_kana::ToF32;
use bevy_render::extract_component::ExtractComponent;
use bevy_render::render_graph::RenderLabel;
use bevy_render::sync_world::MainEntity;
use bevy_render::sync_world::MainEntityHashMap;

/// Tracks which outline infrastructure is needed this frame.
/// Derived from the extracted outline cache to gate expensive hull resources.
#[derive(Resource, Default)]
pub(super) struct ActiveOutlineModes {
    /// Which outline methods are active this frame.
    pub(super) methods: ActiveOutlineMethods,
}

/// Render-world cache of all extracted outlines, keyed by main-world entity.
#[derive(Resource, Default)]
pub(super) struct ExtractedOutlineUniforms {
    /// Map from main-world entity to its extracted outline data.
    pub(super) by_main_entity: MainEntityHashMap<ExtractedOutline>,
    /// Which outline methods appear in the extracted cache.
    pub(super) methods:        ActiveOutlineMethods,
    /// Largest JFA outline width across all extracted outlines.
    pub(super) max_jfa_width:  f32,
}

impl ExtractedOutlineUniforms {
    pub(super) fn upsert(&mut self, entity: MainEntity, outline: ExtractedOutline) -> bool {
        if let Some(existing) = self.by_main_entity.get_mut(&entity) {
            if *existing == outline {
                return false;
            }
            *existing = outline;
            return true;
        }

        self.by_main_entity.insert(entity, outline);
        true
    }

    pub(super) fn recompute_flags_and_width(&mut self) {
        self.methods = ActiveOutlineMethods::None;
        self.max_jfa_width = 0.0;

        for outline in self.by_main_entity.values() {
            self.methods = self.methods.with_outline_method(outline.mode);
            if outline.mode == OutlineMethod::JumpFlood {
                self.max_jfa_width = self.max_jfa_width.max(outline.width);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum ActiveOutlineMethods {
    #[default]
    None,
    JumpFloodOnly,
    HullOnly,
    JumpFloodAndHull,
}

impl ActiveOutlineMethods {
    pub(super) const fn has_jfa(self) -> bool {
        matches!(self, Self::JumpFloodOnly | Self::JumpFloodAndHull)
    }

    pub(super) const fn has_hull(self) -> bool {
        matches!(self, Self::HullOnly | Self::JumpFloodAndHull)
    }

    pub(super) const fn with_outline_method(self, method: OutlineMethod) -> Self {
        let includes_jfa = self.has_jfa() || matches!(method, OutlineMethod::JumpFlood);
        let includes_hull = self.has_hull()
            || matches!(method, OutlineMethod::WorldHull | OutlineMethod::ScreenHull);

        match (includes_jfa, includes_hull) {
            (false, false) => Self::None,
            (true, false) => Self::JumpFloodOnly,
            (false, true) => Self::HullOnly,
            (true, true) => Self::JumpFloodAndHull,
        }
    }
}

/// Marker component that prevents outline propagation to this entity.
///
/// When a parent entity has an outline that propagates to descendant `Mesh3d` entities,
/// any descendant with `NoOutline` will be skipped. This is useful for invisible helper
/// meshes (e.g. backside pick planes with `AlphaMode::Blend`) that should never receive
/// an outline, even when their ancestor is outlined.
///
/// # Example
///
/// ```rust,no_run
/// # use bevy::prelude::*;
/// # use bevy_liminal::NoOutline;
/// // Invisible pick plane that should not receive outline propagation
/// commands.spawn((
///     Name::new("Backside Pick Plane"),
///     Mesh3d(mesh),
///     MeshMaterial3d(transparent_material),
///     NoOutline,
/// ));
/// ```
#[derive(Debug, Component, Reflect, Clone, Copy, Default)]
#[reflect(Component)]
pub struct NoOutline;

/// Marker component for enabling a 3D camera to render mesh outlines.
#[derive(Debug, Component, Reflect, Clone, ExtractComponent)]
#[reflect(Component)]
#[require(DepthPrepass)]
pub struct OutlineCamera;

/// Adds a mesh outline effect to an entity with a `Mesh3d` component.
///
/// Construct via one of the three named constructors — each returns a type-safe
/// builder that only exposes settings valid for that method.
///
/// # Example
///
/// ```rust,no_run
/// # use bevy::prelude::*;
/// # use bevy_liminal::Outline;
/// # use bevy_liminal::OverlapMode;
/// // JFA — screen-space silhouette, works on all geometry
/// Outline::jump_flood(4.0).with_color(Color::WHITE).build();
///
/// // ScreenHull — pixel-width vertex extrusion for 3D meshes
/// Outline::screen_hull(3.0)
///     .with_overlap(OverlapMode::PerMesh)
///     .build();
///
/// // WorldHull — world-unit vertex extrusion for 3D meshes
/// Outline::world_hull(0.05)
///     .with_overlap(OverlapMode::Grouped)
///     .build();
/// ```
#[derive(Debug, Component, Reflect, Clone)]
#[reflect(Component)]
pub struct Outline {
    /// Outline width. Pixels for `JumpFlood`/`ScreenHull`, world units for `WorldHull`.
    pub width:               f32,
    /// Outline color.
    pub color:               Color,
    /// Multiplier applied to `color` in the shader. Values > 1.0 produce HDR glow via bloom.
    pub intensity:           f32,
    /// Which algorithm to use. See `OutlineMethod` for guidance.
    pub mode:                OutlineMethod,
    /// How overlapping outlines from different entities interact.
    pub overlap:             OverlapMode,
    /// Line style (currently only `Solid`).
    pub style:               LineStyle,
    /// Whether this outline participates in extraction and rendering.
    pub activity:            OutlineActivity,
    /// Set internally by propagation. When `Grouped`, all propagated children share this
    /// entity's ID as the owner for overlap resolution. Not user-facing.
    pub(crate) group_source: Option<Entity>,
}

impl Outline {
    /// Create a JFA outline builder. Width is in pixels.
    #[must_use]
    pub const fn jump_flood(
        width: f32,
    ) -> super::outline_builder::OutlineBuilder<super::outline_builder::JumpFloodState> {
        super::outline_builder::OutlineBuilder::jump_flood(width)
    }

    /// Create a screen-space hull outline builder. Width is in pixels.
    #[must_use]
    pub const fn screen_hull(
        width: f32,
    ) -> super::outline_builder::OutlineBuilder<super::outline_builder::ScreenHullState> {
        super::outline_builder::OutlineBuilder::screen_hull(width)
    }

    /// Create a world-space hull outline builder. Width is in world units.
    #[must_use]
    pub const fn world_hull(
        width: f32,
    ) -> super::outline_builder::OutlineBuilder<super::outline_builder::WorldHullState> {
        super::outline_builder::OutlineBuilder::world_hull(width)
    }
}

/// Which outline algorithm to use.
///
/// - `JumpFlood`: Screen-space silhouette expansion. Works on **all** geometry including flat
///   panels and UI planes. Width is in pixels.
/// - `WorldHull`: Vertex extrusion with world-unit width. Best for 3D volumetric meshes where
///   outline thickness should scale with distance.
/// - `ScreenHull`: Vertex extrusion with pixel width. Best for 3D volumetric meshes where outline
///   thickness should remain constant on screen.
#[derive(Debug, Clone, Copy, Reflect, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum OutlineMethod {
    /// Screen-space silhouette expansion via jump-flood algorithm. Width is in pixels.
    #[default]
    JumpFlood,
    /// Vertex extrusion with world-unit width that scales with camera distance.
    WorldHull,
    /// Vertex extrusion with pixel-unit width that stays constant on screen.
    ScreenHull,
}

/// How overlapping outlines from different entities interact.
///
/// **Note:** `OverlapMode` only affects hull methods (`WorldHull`/`ScreenHull`). JFA always
/// produces merged outlines regardless of this setting.
///
/// - `Merged`: Overlapping outlined meshes share a single unified silhouette outline. No outline is
///   drawn where two outlined surfaces overlap — they merge into one shape.
///
/// - `Grouped`: All meshes within the same entity hierarchy (parent + children sharing a
///   `group_owner`) merge into one outline, but that group is visually distinct from other groups.
///   A cube with child spheres looks like one outlined unit, while a neighboring torus has its own
///   separate outline.
///
/// - `PerMesh`: Every individual `Mesh3d` gets its own distinct outline boundary, even if it's a
///   child of a larger entity. Child spheres inside a cube each show their own outline.
#[derive(Debug, Clone, Copy, Reflect, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum OverlapMode {
    /// Overlapping outlines merge into one shared silhouette.
    #[default]
    Merged,
    /// Meshes in the same group (via `group_owner`) merge, but are distinct from other groups.
    Grouped,
    /// Every individual mesh gets its own outline boundary.
    PerMesh,
}

/// Visual style of the outline stroke.
#[derive(Debug, Clone, Copy, Reflect, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum LineStyle {
    /// A continuous solid stroke.
    #[default]
    Solid,
}

/// Whether an `Outline` is active without removing the component.
#[derive(Debug, Clone, Copy, Reflect, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum OutlineActivity {
    /// The outline participates in extraction and rendering.
    #[default]
    Enabled,
    /// The outline is present but skipped during extraction.
    Disabled,
}

impl OutlineActivity {
    /// Returns whether the outline should participate in extraction and rendering.
    #[must_use]
    pub const fn is_enabled(self) -> bool { matches!(self, Self::Enabled) }
}

impl OverlapMode {
    /// Returns the shader factor for this overlap mode (0.0 for `Merged`, 1.0 otherwise).
    #[must_use]
    pub const fn as_shader_factor(self) -> f32 {
        match self {
            Self::Merged => 0.0,
            Self::Grouped | Self::PerMesh => 1.0,
        }
    }
}

/// GPU-ready outline data extracted from the main world.
#[derive(Debug, Reflect, Clone, PartialEq)]
pub(super) struct ExtractedOutline {
    /// Color multiplier for HDR glow via bloom.
    pub(super) intensity: f32,
    /// Outline width in pixels or world units depending on `mode`.
    pub(super) width:     f32,
    /// Draw priority for ordering (reserved for future use).
    pub(super) priority:  f32,
    /// Shader overlap factor derived from `OverlapMode`.
    pub(super) overlap:   f32,
    /// Unique owner ID used for per-mesh and grouped overlap resolution.
    pub(super) owner_id:  f32,
    /// Linear RGBA outline color as a `Vec4`.
    pub(super) color:     Vec4,
    /// Which outline algorithm this entity uses.
    pub(super) mode:      OutlineMethod,
}

impl ExtractedOutline {
    pub(super) fn from_main_world(entity: Entity, outline: &Outline) -> Self {
        let linear_color: LinearRgba = outline.color.into();
        let owner_entity = match outline.overlap {
            OverlapMode::Grouped => outline.group_source.unwrap_or(entity),
            _ => entity,
        };
        Self {
            intensity: outline.intensity,
            width:     outline.width,
            priority:  0.0,
            overlap:   outline.overlap.as_shader_factor(),
            owner_id:  owner_entity.index().index().to_f32() + 1.0,
            color:     linear_color.to_vec4(),
            mode:      outline.mode,
        }
    }
}

/// Render graph label for the outline pass.
#[derive(Copy, Clone, Debug, RenderLabel, Hash, PartialEq, Eq)]
pub(super) enum OutlineRenderGraphNode {
    /// The main outline render node that runs mask, flood, hull, and compose sub-passes.
    OutlineNode,
}

/// Ensures the main pass depth texture has `TEXTURE_BINDING` so the compose shader
/// can sample it for correct occlusion of transmissive/transparent geometry.
///
/// Needs to run in the main app because `Camera3d::depth_texture_usages` controls
/// how the GPU texture is allocated — by the time extraction runs, it's too late.
///
/// See `bevy_pbr::atmosphere::configure_camera_depth_usages` for the same pattern in Bevy.
pub(super) fn configure_outline_camera_depth_texture(
    mut cameras: Query<&mut Camera3d, With<OutlineCamera>>,
) {
    for mut camera_3d in &mut cameras {
        let mut usages = TextureUsages::from(camera_3d.depth_texture_usages);
        usages |= TextureUsages::TEXTURE_BINDING;
        camera_3d.depth_texture_usages = usages.into();
    }
}
