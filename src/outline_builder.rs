use std::marker::PhantomData;

use bevy::prelude::Color;
use bevy::prelude::Entity;

use crate::LineStyle;
use crate::Outline;
use crate::OutlineMethod;
use crate::OverlapMode;

pub trait OutlineModeState: private::Sealed {
    const MODE: OutlineMethod;
}

pub trait HullModeState: OutlineModeState {}

#[derive(Debug, Clone, Copy)]
pub struct JumpFloodState;

#[derive(Debug, Clone, Copy)]
pub struct WorldHullState;

#[derive(Debug, Clone, Copy)]
pub struct ScreenHullState;

impl OutlineModeState for JumpFloodState {
    const MODE: OutlineMethod = OutlineMethod::JumpFlood;
}

impl OutlineModeState for WorldHullState {
    const MODE: OutlineMethod = OutlineMethod::WorldHull;
}

impl OutlineModeState for ScreenHullState {
    const MODE: OutlineMethod = OutlineMethod::ScreenHull;
}

impl HullModeState for WorldHullState {}
impl HullModeState for ScreenHullState {}

#[derive(Debug, Clone)]
pub struct OutlineBuilder<M: OutlineModeState> {
    width:       f32,
    intensity:   f32,
    color:       Color,
    overlap:     OverlapMode,
    group_owner: Option<Entity>,
    _mode:       PhantomData<M>,
}

fn defaults<M: OutlineModeState>(width: f32) -> OutlineBuilder<M> {
    OutlineBuilder {
        width,
        intensity: 1.0,
        color: Color::BLACK,
        overlap: OverlapMode::Merged,
        group_owner: None,
        _mode: PhantomData,
    }
}

impl OutlineBuilder<JumpFloodState> {
    pub fn jump_flood(width: f32) -> Self { defaults(width) }

    pub fn build(self) -> Outline {
        Outline {
            intensity:   self.intensity,
            width:       self.width,
            overlap:     OverlapMode::Merged,
            group_owner: None,
            color:       self.color,
            mode:        OutlineMethod::JumpFlood,
            style:       LineStyle::Solid,
            enabled:     true,
        }
    }
}

impl OutlineBuilder<WorldHullState> {
    pub fn world_hull(width: f32) -> Self { defaults(width) }
}

impl OutlineBuilder<ScreenHullState> {
    pub fn screen_hull(width: f32) -> Self { defaults(width) }
}

/// Settings available on all outline methods.
impl<M: OutlineModeState> OutlineBuilder<M> {
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

/// Settings only available on hull methods (`WorldHull`, `ScreenHull`).
impl<M: HullModeState> OutlineBuilder<M> {
    pub fn with_overlap(mut self, overlap: OverlapMode) -> Self {
        self.overlap = overlap;
        self
    }

    pub fn with_group_owner(mut self, owner: Entity) -> Self {
        self.group_owner = Some(owner);
        self
    }

    pub fn build(self) -> Outline {
        Outline {
            intensity:   self.intensity,
            width:       self.width,
            overlap:     self.overlap,
            group_owner: self.group_owner,
            color:       self.color,
            mode:        M::MODE,
            style:       LineStyle::Solid,
            enabled:     true,
        }
    }
}

mod private {
    pub trait Sealed {}

    impl Sealed for super::JumpFloodState {}
    impl Sealed for super::WorldHullState {}
    impl Sealed for super::ScreenHullState {}
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Color;

    use crate::Outline;
    use crate::OutlineMethod;
    use crate::OverlapMode;

    #[test]
    fn jump_flood_builds_correctly() {
        let outline = Outline::jump_flood(4.0).with_color(Color::WHITE).build();

        assert_eq!(outline.mode, OutlineMethod::JumpFlood);
        assert_eq!(outline.width, 4.0);
        assert_eq!(outline.overlap, OverlapMode::Merged);
        assert!(outline.enabled);
    }

    #[test]
    fn screen_hull_with_overlap() {
        let outline = Outline::screen_hull(3.0)
            .with_overlap(OverlapMode::PerMesh)
            .build();

        assert_eq!(outline.mode, OutlineMethod::ScreenHull);
        assert_eq!(outline.width, 3.0);
        assert_eq!(outline.overlap, OverlapMode::PerMesh);
    }

    #[test]
    fn world_hull_with_grouped_overlap() {
        let outline = Outline::world_hull(0.05)
            .with_overlap(OverlapMode::Grouped)
            .build();

        assert_eq!(outline.mode, OutlineMethod::WorldHull);
        assert_eq!(outline.overlap, OverlapMode::Grouped);
    }

    #[test]
    fn enabled_defaults_to_true() {
        let outline = Outline::jump_flood(2.0).build();
        assert!(outline.enabled);
    }
}
