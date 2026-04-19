use bevy::prelude::Resource;
use bevy_render::sync_world::MainEntity;
use bevy_render::sync_world::MainEntityHashMap;

use super::outline_api::OutlineMethod;
use super::render_data::ExtractedOutline;

/// Tracks which outline infrastructure is needed this frame.
/// Derived from the extracted outline cache to gate expensive hull resources.
#[derive(Resource, Default)]
pub(crate) struct ActiveOutlineModes {
    /// Which outline methods are active this frame.
    pub(crate) methods: ActiveOutlineMethods,
}

/// Render-world cache of all extracted outlines, keyed by main-world entity.
#[derive(Resource, Default)]
pub(crate) struct ExtractedOutlineUniforms {
    /// Map from main-world entity to its extracted outline data.
    pub(crate) by_main_entity: MainEntityHashMap<ExtractedOutline>,
    /// Which outline methods appear in the extracted cache.
    pub(crate) methods:        ActiveOutlineMethods,
    /// Largest JFA outline width across all extracted outlines.
    pub(crate) max_jfa_width:  f32,
}

impl ExtractedOutlineUniforms {
    pub(crate) fn upsert(&mut self, entity: MainEntity, outline: ExtractedOutline) -> bool {
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

    pub(crate) fn recompute_flags_and_width(&mut self) {
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
pub(crate) enum ActiveOutlineMethods {
    #[default]
    None,
    JumpFloodOnly,
    HullOnly,
    JumpFloodAndHull,
}

impl ActiveOutlineMethods {
    pub(crate) const fn has_jfa(self) -> bool {
        matches!(self, Self::JumpFloodOnly | Self::JumpFloodAndHull)
    }

    pub(crate) const fn has_hull(self) -> bool {
        matches!(self, Self::HullOnly | Self::JumpFloodAndHull)
    }

    pub(crate) const fn with_outline_method(self, method: OutlineMethod) -> Self {
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
