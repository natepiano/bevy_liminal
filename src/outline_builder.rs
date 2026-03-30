use std::marker::PhantomData;

use bevy::prelude::Color;

use crate::LineStyle;
use crate::Outline;
use crate::OutlineMethod;
use crate::OverlapMode;

/// Sealed trait implemented by outline mode type-state markers.
pub trait OutlineModeState: private::Sealed {
    const MODE: OutlineMethod;
}

/// Marker trait for hull-based outline modes (`WorldHull`, `ScreenHull`).
pub trait HullModeState: OutlineModeState {}

/// Type-state marker for the jump-flood outline method.
#[derive(Debug, Clone, Copy)]
pub struct JumpFloodState;

/// Type-state marker for the world-space hull outline method.
#[derive(Debug, Clone, Copy)]
pub struct WorldHullState;

/// Type-state marker for the screen-space hull outline method.
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

/// Type-safe builder for constructing an `Outline` component.
#[derive(Debug, Clone)]
pub struct OutlineBuilder<M: OutlineModeState> {
    width:     f32,
    intensity: f32,
    color:     Color,
    overlap:   OverlapMode,
    _mode:     PhantomData<M>,
}

const fn defaults<M: OutlineModeState>(width: f32) -> OutlineBuilder<M> {
    OutlineBuilder {
        width,
        intensity: 1.0,
        color: Color::BLACK,
        overlap: OverlapMode::Merged,
        _mode: PhantomData,
    }
}

impl OutlineBuilder<JumpFloodState> {
    /// Create a new jump-flood outline builder with the given pixel width.
    #[must_use]
    pub const fn jump_flood(width: f32) -> Self { defaults(width) }

    /// Consume the builder and produce a configured `Outline` component.
    #[must_use]
    pub const fn build(self) -> Outline {
        Outline {
            intensity:    self.intensity,
            width:        self.width,
            overlap:      OverlapMode::Merged,
            group_source: None,
            color:        self.color,
            mode:         OutlineMethod::JumpFlood,
            style:        LineStyle::Solid,
            enabled:      true,
        }
    }
}

impl OutlineBuilder<WorldHullState> {
    /// Create a new world-space hull outline builder with the given world-unit width.
    #[must_use]
    pub const fn world_hull(width: f32) -> Self { defaults(width) }
}

impl OutlineBuilder<ScreenHullState> {
    /// Create a new screen-space hull outline builder with the given pixel width.
    #[must_use]
    pub const fn screen_hull(width: f32) -> Self { defaults(width) }
}

/// Settings available on all outline methods.
impl<M: OutlineModeState> OutlineBuilder<M> {
    /// Override the outline width.
    #[must_use]
    pub const fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Set the color intensity multiplier (values > 1.0 produce HDR glow).
    #[must_use]
    pub const fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// Set the outline color.
    #[must_use]
    pub const fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

/// Settings only available on hull methods (`WorldHull`, `ScreenHull`).
impl<M: HullModeState> OutlineBuilder<M> {
    /// Set the overlap mode for hull outlines.
    #[must_use]
    pub const fn with_overlap(mut self, overlap: OverlapMode) -> Self {
        self.overlap = overlap;
        self
    }

    /// Consume the builder and produce a configured `Outline` component.
    #[must_use]
    pub const fn build(self) -> Outline {
        Outline {
            intensity:    self.intensity,
            width:        self.width,
            overlap:      self.overlap,
            color:        self.color,
            mode:         M::MODE,
            style:        LineStyle::Solid,
            enabled:      true,
            group_source: None,
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
        assert!((outline.width - 4.0).abs() < f32::EPSILON);
        assert_eq!(outline.overlap, OverlapMode::Merged);
        assert!(outline.enabled);
    }

    #[test]
    fn screen_hull_with_overlap() {
        let outline = Outline::screen_hull(3.0)
            .with_overlap(OverlapMode::PerMesh)
            .build();

        assert_eq!(outline.mode, OutlineMethod::ScreenHull);
        assert!((outline.width - 3.0).abs() < f32::EPSILON);
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
