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
    pub has_jfa:  bool,
    pub has_hull: bool,
}

#[derive(Resource, Default)]
pub struct ExtractedOutlineUniforms {
    pub by_main_entity: MainEntityHashMap<ExtractedOutline>,
    pub has_jfa:        bool,
    pub has_hull:       bool,
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
        dirty |= extracted_outlines.upsert(
            MainEntity::from(entity),
            ExtractedOutline::from_main_world(entity, outline),
        );
    }

    for (entity, outline) in &added_mesh_outlines {
        dirty |= extracted_outlines.upsert(
            MainEntity::from(entity),
            ExtractedOutline::from_main_world(entity, outline),
        );
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

pub struct LiminalPlugin;

impl Plugin for LiminalPlugin {
    fn build(&self, app: &mut App) {
        load_shaders(app);

        app.add_plugins((ExtractComponentPlugin::<OutlineCamera>::default(),));
        app.register_type::<Outline>();
        app.register_type::<OutlineMethod>();
        app.register_type::<OverlapMode>();

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

/// Adds a mesh outline effect to entity.
/// Should be added to the entity containing the Mesh3d component.
#[derive(Debug, Component, Reflect, Clone)]
#[reflect(Component)]
pub struct Outline {
    pub intensity: f32,
    pub width:     f32,
    pub priority:  f32,
    pub overlap:   OverlapMode,
    pub color:     Color,
    pub mode:      OutlineMethod,
}

impl Outline {
    pub fn new(width: f32) -> Self {
        Self {
            intensity: 1.0,
            width,
            priority: 0.0,
            overlap: OverlapMode::Merged,
            color: Color::BLACK,
            mode: OutlineMethod::JumpFlood,
        }
    }

    pub fn builder(width: f32) -> OutlineBuilder<JumpFloodState> {
        OutlineBuilder::jump_flood(width)
    }

    pub fn with_intensity(self, intensity: f32) -> Self { Self { intensity, ..self } }

    pub fn with_priority(self, priority: f32) -> Self { Self { priority, ..self } }

    pub fn with_color(self, color: Color) -> Self { Self { color, ..self } }

    pub fn with_overlap(self, overlap: OverlapMode) -> Self { Self { overlap, ..self } }

    pub fn with_mode(self, mode: OutlineMethod) -> Self { Self { mode, ..self } }
}

#[derive(Debug, Clone, Copy, Reflect, PartialEq, Eq, Default)]
pub enum OutlineMethod {
    #[default]
    JumpFlood,
    WorldHull,
    ScreenHull,
}

#[derive(Debug, Clone, Copy, Reflect, PartialEq, Eq, Default)]
pub enum OverlapMode {
    #[default]
    Merged,
    Individual,
}

impl OverlapMode {
    pub fn as_shader_factor(self) -> f32 {
        match self {
            OverlapMode::Merged => 0.0,
            OverlapMode::Individual => 1.0,
        }
    }
}

#[derive(Debug, Reflect, Clone, PartialEq)]
pub struct ExtractedOutline {
    pub intensity: f32,
    pub width:     f32,
    pub priority:  f32,
    pub overlap:   f32,
    pub owner_id:  f32,
    pub color:     Vec4,
    pub mode:      OutlineMethod,
}

impl ExtractedOutline {
    fn from_main_world(entity: Entity, outline: &Outline) -> Self {
        let linear_color: LinearRgba = outline.color.into();
        ExtractedOutline {
            intensity: outline.intensity,
            width:     outline.width,
            priority:  outline.priority,
            overlap:   outline.overlap.as_shader_factor(),
            owner_id:  entity.index().index() as f32 + 1.0,
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

#[derive(Copy, Clone, Debug, RenderLabel, Hash, PartialEq, Eq)]
pub enum OutlineRenderGraphNode {
    OutlineNode,
}
