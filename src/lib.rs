//! Bevy plugin for rendering mesh outlines using jump-flood and hull-extrusion methods.

mod compose;
mod flood;
mod hull_pipeline;
mod mask;
mod mask_pipeline;
mod node;
mod outline_builder;
mod queue;
mod render;
mod shaders;
mod texture;
mod uniforms;
mod view;

use bevy::core_pipeline::core_3d::graph::Core3d;
use bevy::core_pipeline::core_3d::graph::Node3d;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::pbr::DrawMesh;
use bevy::pbr::SetMeshBindGroup;
use bevy::pbr::SetMeshViewBindGroup;
use bevy::pbr::SetMeshViewBindingArrayBindGroup;
use bevy::pbr::extract_skins;
use bevy::prelude::*;
use bevy::scene::SceneInstanceReady;
use bevy_kana::ToF32;
use bevy_render::Extract;
use bevy_render::Render;
use bevy_render::RenderApp;
use bevy_render::RenderDebugFlags;
use bevy_render::RenderSystems;
use bevy_render::extract_component::ExtractComponent;
use bevy_render::extract_component::ExtractComponentPlugin;
use bevy_render::render_graph::RenderGraphExt;
use bevy_render::render_graph::RenderLabel;
use bevy_render::render_graph::ViewNodeRunner;
use bevy_render::render_phase::AddRenderCommand;
use bevy_render::render_phase::BinnedRenderPhasePlugin;
use bevy_render::render_phase::DrawFunctions;
use bevy_render::render_phase::SetItemPipeline;
use bevy_render::render_phase::ViewBinnedRenderPhases;
use bevy_render::render_resource::GpuArrayBuffer;
use bevy_render::render_resource::SpecializedMeshPipelines;
use bevy_render::render_resource::TextureUsages;
use bevy_render::renderer::RenderDevice;
use bevy_render::sync_world::MainEntity;
use bevy_render::sync_world::MainEntityHashMap;
use compose::ComposeOutputPipeline;
use flood::JumpFloodPipeline;
use flood::prepare_flood_settings;
use hull_pipeline::HullPipeline;
use mask::HullOutlinePhase;
use mask::JfaOutlinePhase;
use mask_pipeline::MeshMaskPipeline;
use node::OutlineNode;
pub use outline_builder::JumpFloodState;
pub use outline_builder::OutlineBuilder;
pub use outline_builder::ScreenHullState;
pub use outline_builder::WorldHullState;
use queue::queue_hull_outline;
use queue::queue_outline;
use render::HullOutlineBindGroup;
use render::HullOutlineUniformBuffer;
use render::OutlineBindGroup;
use render::OutlineUniformBuffer;
use render::SetHullDepthBindGroup;
use render::SetHullOutlineBindGroup;
use render::SetOutlineBindGroup;
use render::prepare_hull_depth_view_bind_groups;
use render::prepare_hull_outline_bind_group;
use render::prepare_hull_outline_buffer;
use render::prepare_outline_bind_group;
use render::prepare_outline_buffer;
use texture::prepare_flood_textures;
use view::update_views;

use crate::shaders::load_shaders;

pub(crate) type DrawOutline = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetMeshBindGroup<2>,
    SetOutlineBindGroup<3>,
    DrawMesh,
);

pub(crate) type DrawHull = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetMeshBindGroup<2>,
    SetHullOutlineBindGroup<3>,
    SetHullDepthBindGroup<4>,
    DrawMesh,
);

/// Tracks which outline infrastructure is needed this frame.
/// Derived from the extracted outline cache to gate expensive hull resources.
#[derive(Resource, Default)]
pub struct ActiveOutlineModes {
    /// Whether any entity requires jump-flood outlines.
    pub has_jfa:  bool,
    /// Whether any entity requires hull-extrusion outlines.
    pub has_hull: bool,
}

/// Render-world cache of all extracted outlines, keyed by main-world entity.
#[derive(Resource, Default)]
pub struct ExtractedOutlineUniforms {
    /// Map from main-world entity to its extracted outline data.
    pub by_main_entity: MainEntityHashMap<ExtractedOutline>,
    /// Whether any extracted outline uses the JFA method.
    pub has_jfa:        bool,
    /// Whether any extracted outline uses a hull method.
    pub has_hull:       bool,
    /// Largest JFA outline width across all extracted outlines.
    pub max_jfa_width:  f32,
}

impl ExtractedOutlineUniforms {
    fn upsert(&mut self, entity: MainEntity, outline: ExtractedOutline) -> bool {
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

    fn recompute_flags_and_width(&mut self) {
        self.has_jfa = false;
        self.has_hull = false;
        self.max_jfa_width = 0.0;

        for outline in self.by_main_entity.values() {
            match outline.mode {
                OutlineMethod::JumpFlood => {
                    self.has_jfa = true;
                    self.max_jfa_width = self.max_jfa_width.max(outline.width);
                },
                _ => self.has_hull = true,
            }
        }
    }
}

fn extract_outline_uniforms(
    mut extracted_outlines: ResMut<ExtractedOutlineUniforms>,
    added_or_changed_outlines: Extract<Query<OutlineEntityAndOutline, AddedOrChangedOutlineFilter>>,
    added_mesh_outlines: Extract<Query<OutlineEntityAndOutline, AddedOutlineFilter>>,
    mut removed_outlines: Extract<RemovedComponents<Outline>>,
    mut removed_meshes: Extract<RemovedComponents<Mesh3d>>,
) {
    let mut dirty = false;

    for entity in removed_outlines.read() {
        dirty |= extracted_outlines
            .by_main_entity
            .remove(&MainEntity::from(entity))
            .is_some();
    }

    for entity in removed_meshes.read() {
        dirty |= extracted_outlines
            .by_main_entity
            .remove(&MainEntity::from(entity))
            .is_some();
    }

    for (entity, outline) in &added_or_changed_outlines {
        if outline.enabled {
            dirty |= extracted_outlines.upsert(
                MainEntity::from(entity),
                ExtractedOutline::from_main_world(entity, outline),
            );
        } else {
            dirty |= extracted_outlines
                .by_main_entity
                .remove(&MainEntity::from(entity))
                .is_some();
        }
    }

    for (entity, outline) in &added_mesh_outlines {
        if outline.enabled {
            dirty |= extracted_outlines.upsert(
                MainEntity::from(entity),
                ExtractedOutline::from_main_world(entity, outline),
            );
        }
    }

    if dirty {
        extracted_outlines.recompute_flags_and_width();
    }
}

type OutlineEntityAndOutline = (Entity, &'static Outline);
type AddedOrChangedOutlineFilter = (With<Mesh3d>, Or<(Added<Outline>, Changed<Outline>)>);
type AddedOutlineFilter = (Added<Mesh3d>, With<Outline>);

fn update_active_outline_modes(
    extracted_outlines: Res<ExtractedOutlineUniforms>,
    mut active: ResMut<ActiveOutlineModes>,
) {
    active.has_jfa = extracted_outlines.has_jfa;
    active.has_hull = extracted_outlines.has_hull;
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

/// Bevy plugin that registers outline rendering systems, pipelines, and render graph nodes.
pub struct LiminalPlugin;

impl Plugin for LiminalPlugin {
    fn build(&self, app: &mut App) {
        load_shaders(app);

        app.add_plugins((ExtractComponentPlugin::<OutlineCamera>::default(),));
        app.register_type::<Outline>();
        app.register_type::<OutlineMethod>();
        app.register_type::<OverlapMode>();
        app.register_type::<NoOutline>();

        // Propagation observers
        app.add_observer(propagate_outline_to_descendants);
        app.add_observer(propagate_outline_on_child_added);
        app.add_observer(propagate_outline_on_mesh_added);
        app.add_observer(propagate_outline_on_scene_ready);
        app.add_observer(remove_outline_from_descendants);

        // Change detection for propagated outlines
        app.add_systems(PostUpdate, sync_propagated_outlines);

        // Ensure the main pass depth texture has TEXTURE_BINDING so the compose
        // shader can sample it for correct occlusion of transmissive/transparent geometry.
        app.add_systems(PostUpdate, configure_outline_camera_depth_texture);

        app.add_plugins((
            BinnedRenderPhasePlugin::<JfaOutlinePhase, MeshMaskPipeline>::new(
                RenderDebugFlags::default(),
            ),
            BinnedRenderPhasePlugin::<HullOutlinePhase, HullPipeline>::new(
                RenderDebugFlags::default(),
            ),
        ));

        app.sub_app_mut(RenderApp)
            .init_resource::<DrawFunctions<JfaOutlinePhase>>()
            .init_resource::<DrawFunctions<HullOutlinePhase>>()
            .init_resource::<SpecializedMeshPipelines<MeshMaskPipeline>>()
            .init_resource::<SpecializedMeshPipelines<HullPipeline>>()
            .init_resource::<ViewBinnedRenderPhases<JfaOutlinePhase>>()
            .init_resource::<ViewBinnedRenderPhases<HullOutlinePhase>>()
            .init_resource::<OutlineBindGroup>()
            .init_resource::<HullOutlineBindGroup>()
            .init_resource::<ActiveOutlineModes>()
            .init_resource::<ExtractedOutlineUniforms>()
            .add_systems(
                ExtractSchedule,
                (extract_outline_uniforms, update_views.after(extract_skins)),
            )
            .add_systems(
                Render,
                (
                    update_active_outline_modes
                        .in_set(RenderSystems::Queue)
                        .before(RenderSystems::QueueMeshes),
                    queue_outline.in_set(RenderSystems::QueueMeshes),
                    queue_hull_outline.in_set(RenderSystems::QueueMeshes),
                    prepare_outline_buffer.in_set(RenderSystems::PrepareResources),
                    prepare_hull_outline_buffer.in_set(RenderSystems::PrepareResources),
                    (
                        prepare_flood_settings,
                        prepare_flood_textures,
                        prepare_outline_bind_group.after(prepare_flood_textures),
                        prepare_hull_outline_bind_group,
                        prepare_hull_depth_view_bind_groups.after(prepare_flood_textures),
                    )
                        .in_set(RenderSystems::PrepareBindGroups),
                ),
            )
            .add_render_command::<JfaOutlinePhase, DrawOutline>()
            .add_render_command::<HullOutlinePhase, DrawHull>()
            .add_render_graph_node::<ViewNodeRunner<OutlineNode>>(
                Core3d,
                OutlineRenderGraphNode::OutlineNode,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPass,
                    OutlineRenderGraphNode::OutlineNode,
                    Node3d::Bloom,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let render_device = render_app.world().resource::<RenderDevice>();
        let outline_uniform_buffer =
            OutlineUniformBuffer(GpuArrayBuffer::new(&render_device.limits()));
        let hull_outline_uniform_buffer =
            HullOutlineUniformBuffer(GpuArrayBuffer::new(&render_device.limits()));

        render_app
            .insert_resource(outline_uniform_buffer)
            .insert_resource(hull_outline_uniform_buffer)
            .init_resource::<MeshMaskPipeline>()
            .init_resource::<HullPipeline>()
            .init_resource::<JumpFloodPipeline>()
            .init_resource::<ComposeOutputPipeline>();
    }
}

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
    /// When `false`, extraction skips this outline without removing the component.
    pub enabled:             bool,
    /// Set internally by propagation. When `Grouped`, all propagated children share this
    /// entity's ID as the owner for overlap resolution. Not user-facing.
    pub(crate) group_source: Option<Entity>,
}

impl Outline {
    /// Create a JFA outline builder. Width is in pixels.
    #[must_use]
    pub const fn jump_flood(width: f32) -> OutlineBuilder<JumpFloodState> {
        OutlineBuilder::jump_flood(width)
    }

    /// Create a screen-space hull outline builder. Width is in pixels.
    #[must_use]
    pub const fn screen_hull(width: f32) -> OutlineBuilder<ScreenHullState> {
        OutlineBuilder::screen_hull(width)
    }

    /// Create a world-space hull outline builder. Width is in world units.
    #[must_use]
    pub const fn world_hull(width: f32) -> OutlineBuilder<WorldHullState> {
        OutlineBuilder::world_hull(width)
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
pub struct ExtractedOutline {
    /// Color multiplier for HDR glow via bloom.
    pub intensity: f32,
    /// Outline width in pixels or world units depending on `mode`.
    pub width:     f32,
    /// Draw priority for ordering (reserved for future use).
    pub priority:  f32,
    /// Shader overlap factor derived from `OverlapMode`.
    pub overlap:   f32,
    /// Unique owner ID used for per-mesh and grouped overlap resolution.
    pub owner_id:  f32,
    /// Linear RGBA outline color as a `Vec4`.
    pub color:     Vec4,
    /// Which outline algorithm this entity uses.
    pub mode:      OutlineMethod,
}

impl ExtractedOutline {
    fn from_main_world(entity: Entity, outline: &Outline) -> Self {
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

/// Ensures the main pass depth texture has `TEXTURE_BINDING` so the compose shader
/// can sample it for correct occlusion of transmissive/transparent geometry.
///
/// Needs to run in the main app because `Camera3d::depth_texture_usages` controls
/// how the GPU texture is allocated — by the time extraction runs, it's too late.
///
/// See `bevy_pbr::atmosphere::configure_camera_depth_usages` for the same pattern in Bevy.
fn configure_outline_camera_depth_texture(mut cameras: Query<&mut Camera3d, With<OutlineCamera>>) {
    for mut camera_3d in &mut cameras {
        let mut usages = TextureUsages::from(camera_3d.depth_texture_usages);
        usages |= TextureUsages::TEXTURE_BINDING;
        camera_3d.depth_texture_usages = usages.into();
    }
}

// --- Propagation ---

/// When `Outline` is added to an entity, propagate it to all descendant `Mesh3d` entities.
/// Skips entities with `NoOutline`. Sets `group_source` for `Grouped` overlap mode.
fn propagate_outline_to_descendants(
    added: On<Add, Outline>,
    outline_query: Query<&Outline>,
    mesh_query: Query<(), (With<Mesh3d>, Without<NoOutline>)>,
    children_query: Query<&Children>,
    mut commands: Commands,
) {
    let source = added.entity;
    let Ok(outline) = outline_query.get(source) else {
        return;
    };

    // Don't re-propagate from entities that received their outline via propagation
    if outline.group_source.is_some() {
        return;
    }

    let mut propagated = outline.clone();
    propagated.group_source = Some(source);

    for descendant in children_query.iter_descendants(source) {
        if mesh_query.contains(descendant) {
            commands.entity(descendant).insert(propagated.clone());
        }
    }
}

/// When a new child is added to the hierarchy, check if any ancestor has `Outline`
/// and propagate it. Handles glTF scene loading where children spawn after the parent.
fn propagate_outline_on_child_added(
    added: On<Add, ChildOf>,
    child_mesh_query: Query<(), (With<Mesh3d>, Without<NoOutline>)>,
    outline_query: Query<&Outline>,
    parent_query: Query<&ChildOf>,
    mut commands: Commands,
) {
    let child = added.entity;
    if !child_mesh_query.contains(child) {
        return;
    }

    // Walk up ancestors to find one with a source `Outline` (not propagated)
    let mut current = child;
    while let Ok(child_of) = parent_query.get(current) {
        let parent = child_of.parent();
        if let Ok(outline) = outline_query.get(parent) {
            // Use the original source if this is a propagated outline
            let source = outline.group_source.unwrap_or(parent);
            let mut propagated = outline.clone();
            propagated.group_source = Some(source);
            commands.entity(child).insert(propagated);
            return;
        }
        current = parent;
    }
}

/// When `Mesh3d` is added to an entity, check if any ancestor has `Outline` and propagate it.
/// Handles glTF scene loading where `Mesh3d` may be added after `ChildOf`.
fn propagate_outline_on_mesh_added(
    added: On<Add, Mesh3d>,
    no_outline_query: Query<(), With<NoOutline>>,
    outline_query: Query<&Outline>,
    parent_query: Query<&ChildOf>,
    existing_outline: Query<(), With<Outline>>,
    mut commands: Commands,
) {
    let child = added.entity;
    if no_outline_query.contains(child) {
        return;
    }
    if existing_outline.contains(child) {
        return;
    }

    // Walk up ancestors to find one with `Outline`
    let mut current = child;
    while let Ok(child_of) = parent_query.get(current) {
        let parent = child_of.parent();
        if let Ok(outline) = outline_query.get(parent) {
            let source = outline.group_source.unwrap_or(parent);
            let mut propagated = outline.clone();
            propagated.group_source = Some(source);
            commands.entity(child).insert(propagated);
            return;
        }
        current = parent;
    }
}

/// When a `SceneInstanceReady` fires on an entity with `Outline`, propagate to
/// all descendant meshes. This handles the `SceneRoot` case where the scene instance
/// entity may not have a `ChildOf` back to the entity with `Outline`.
fn propagate_outline_on_scene_ready(
    ready: On<SceneInstanceReady>,
    outline_query: Query<&Outline>,
    mesh_query: Query<(), (With<Mesh3d>, Without<NoOutline>)>,
    children_query: Query<&Children>,
    mut commands: Commands,
) {
    let source = ready.entity;
    let Ok(outline) = outline_query.get(source) else {
        return;
    };
    if outline.group_source.is_some() {
        return;
    }

    let mut propagated = outline.clone();
    propagated.group_source = Some(source);

    for descendant in children_query.iter_descendants(source) {
        if mesh_query.contains(descendant) {
            commands.entity(descendant).insert(propagated.clone());
        }
    }
}

/// When `Outline` is removed from a source entity, remove it from all descendants.
/// Only acts on source outlines (not propagated copies) to avoid cascading removals.
fn remove_outline_from_descendants(
    removed: On<Remove, Outline>,
    outline_query: Query<&Outline>,
    mesh_query: Query<(), With<Mesh3d>>,
    children_query: Query<&Children>,
    mut commands: Commands,
) {
    let source = removed.entity;

    // Check if any descendant has a propagated outline from this source.
    // If descendants have outlines with a different source (or no source), leave them alone.
    for descendant in children_query.iter_descendants(source) {
        if !mesh_query.contains(descendant) {
            continue;
        }
        if let Ok(desc_outline) = outline_query.get(descendant)
            && desc_outline.group_source == Some(source)
        {
            commands.entity(descendant).try_remove::<Outline>();
        }
    }
}

/// When a source `Outline` changes, update all descendant copies.
fn sync_propagated_outlines(
    changed_outlines: Query<(Entity, &Outline, &Children), Changed<Outline>>,
    mesh_query: Query<(), (With<Mesh3d>, Without<NoOutline>)>,
    children_query: Query<&Children>,
    mut outline_mut: Query<&mut Outline, Without<Children>>,
) {
    for (source, outline, _children) in &changed_outlines {
        // Only sync outlines that are sources (no group_source means this is the original)
        if outline.group_source.is_some() {
            continue;
        }

        let mut propagated = outline.clone();
        propagated.group_source = Some(source);

        for descendant in children_query.iter_descendants(source) {
            if mesh_query.contains(descendant)
                && let Ok(mut desc_outline) = outline_mut.get_mut(descendant)
            {
                *desc_outline = propagated.clone();
            }
        }
    }
}

/// Render graph label for the outline pass.
#[derive(Copy, Clone, Debug, RenderLabel, Hash, PartialEq, Eq)]
pub enum OutlineRenderGraphNode {
    /// The main outline render node that runs mask, flood, hull, and compose sub-passes.
    OutlineNode,
}
