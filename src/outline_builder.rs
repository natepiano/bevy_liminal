use std::marker::PhantomData;

use bevy::prelude::Color;

use crate::MeshOutline;
use crate::OutlineMode;
use crate::OverlapMode;

pub trait OutlineModeState: private::Sealed {
    const MODE: OutlineMode;
}

pub trait HullModeState: OutlineModeState {}

#[derive(Debug, Clone, Copy)]
pub struct JumpFloodState;

#[derive(Debug, Clone, Copy)]
pub struct WorldHullState;

#[derive(Debug, Clone, Copy)]
pub struct ScreenHullState;

impl OutlineModeState for JumpFloodState {
    const MODE: OutlineMode = OutlineMode::JumpFlood;
}

impl OutlineModeState for WorldHullState {
    const MODE: OutlineMode = OutlineMode::WorldHull;
}

impl OutlineModeState for ScreenHullState {
    const MODE: OutlineMode = OutlineMode::ScreenHull;
}

impl HullModeState for WorldHullState {}
impl HullModeState for ScreenHullState {}

#[derive(Debug, Clone)]
pub struct MeshOutlineBuilder<M: OutlineModeState> {
    width:     f32,
    intensity: f32,
    color:     Color,
    priority:  f32,
    overlap:   OverlapMode,
    _mode:     PhantomData<M>,
}

impl MeshOutlineBuilder<JumpFloodState> {
    pub(crate) fn jump_flood(width: f32) -> Self {
        Self {
            intensity: 1.0,
            width,
            priority: 0.0,
            overlap: OverlapMode::Merged,
            color: Color::BLACK,
            _mode: PhantomData,
        }
    }

    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = priority;
        self
    }

    pub fn to_world_hull(self) -> MeshOutlineBuilder<WorldHullState> {
        MeshOutlineBuilder {
            width:     self.width,
            intensity: self.intensity,
            color:     self.color,
            // Priority only applies to JumpFlood and is dropped on transition.
            priority:  0.0,
            overlap:   OverlapMode::Merged,
            _mode:     PhantomData,
        }
    }

    pub fn to_screen_hull(self) -> MeshOutlineBuilder<ScreenHullState> {
        MeshOutlineBuilder {
            width:     self.width,
            intensity: self.intensity,
            color:     self.color,
            // Priority only applies to JumpFlood and is dropped on transition.
            priority:  0.0,
            overlap:   OverlapMode::Merged,
            _mode:     PhantomData,
        }
    }

    pub fn build(self) -> MeshOutline {
        MeshOutline {
            intensity: self.intensity,
            width:     self.width,
            priority:  self.priority,
            overlap:   OverlapMode::Merged,
            color:     self.color,
            mode:      OutlineMode::JumpFlood,
        }
    }
}

impl MeshOutlineBuilder<WorldHullState> {
    pub fn to_jump_flood(self) -> MeshOutlineBuilder<JumpFloodState> {
        MeshOutlineBuilder {
            width:     self.width,
            intensity: self.intensity,
            color:     self.color,
            priority:  0.0,
            // Overlap only applies to hull modes and is dropped on transition.
            overlap:   OverlapMode::Merged,
            _mode:     PhantomData,
        }
    }

    pub fn to_screen_hull(self) -> MeshOutlineBuilder<ScreenHullState> {
        MeshOutlineBuilder {
            width:     self.width,
            intensity: self.intensity,
            color:     self.color,
            priority:  0.0,
            overlap:   self.overlap,
            _mode:     PhantomData,
        }
    }
}

impl MeshOutlineBuilder<ScreenHullState> {
    pub fn to_jump_flood(self) -> MeshOutlineBuilder<JumpFloodState> {
        MeshOutlineBuilder {
            width:     self.width,
            intensity: self.intensity,
            color:     self.color,
            priority:  0.0,
            // Overlap only applies to hull modes and is dropped on transition.
            overlap:   OverlapMode::Merged,
            _mode:     PhantomData,
        }
    }

    pub fn to_world_hull(self) -> MeshOutlineBuilder<WorldHullState> {
        MeshOutlineBuilder {
            width:     self.width,
            intensity: self.intensity,
            color:     self.color,
            priority:  0.0,
            overlap:   self.overlap,
            _mode:     PhantomData,
        }
    }
}

impl<M: OutlineModeState> MeshOutlineBuilder<M> {
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

impl<M: HullModeState> MeshOutlineBuilder<M> {
    pub fn with_overlap(mut self, overlap: OverlapMode) -> Self {
        self.overlap = overlap;
        self
    }

    pub fn build(self) -> MeshOutline {
        MeshOutline {
            intensity: self.intensity,
            width:     self.width,
            priority:  0.0,
            overlap:   self.overlap,
            color:     self.color,
            mode:      M::MODE,
        }
    }
}

impl<M: OutlineModeState> From<MeshOutlineBuilder<M>> for MeshOutline {
    fn from(builder: MeshOutlineBuilder<M>) -> Self {
        match M::MODE {
            OutlineMode::JumpFlood => MeshOutline {
                intensity: builder.intensity,
                width:     builder.width,
                priority:  builder.priority,
                overlap:   OverlapMode::Merged,
                color:     builder.color,
                mode:      OutlineMode::JumpFlood,
            },
            OutlineMode::WorldHull | OutlineMode::ScreenHull => MeshOutline {
                intensity: builder.intensity,
                width:     builder.width,
                priority:  0.0,
                overlap:   builder.overlap,
                color:     builder.color,
                mode:      M::MODE,
            },
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

    use crate::MeshOutline;
    use crate::OutlineMode;
    use crate::OverlapMode;

    #[test]
    fn mode_switches_drop_non_applicable_properties() {
        let outline = MeshOutline::builder(4.0)
            .with_priority(1.0)
            .to_world_hull()
            .with_overlap(OverlapMode::Individual)
            .to_jump_flood()
            .with_intensity(2.0)
            .to_screen_hull()
            .with_color(Color::srgb(1.0, 0.0, 0.0))
            .build();

        assert_eq!(outline.mode, OutlineMode::ScreenHull);
        assert_eq!(outline.width, 4.0);
        assert_eq!(outline.intensity, 2.0);
        assert_eq!(outline.priority, 0.0);
        assert_eq!(outline.overlap, OverlapMode::Merged);
    }

    #[test]
    fn overlap_survives_between_hull_modes() {
        let outline = MeshOutline::builder(3.0)
            .to_world_hull()
            .with_overlap(OverlapMode::Individual)
            .to_screen_hull()
            .build();

        assert_eq!(outline.mode, OutlineMode::ScreenHull);
        assert_eq!(outline.overlap, OverlapMode::Individual);
        assert_eq!(outline.priority, 0.0);
    }

    #[test]
    fn priority_survives_in_jump_flood_only() {
        let jump_outline = MeshOutline::builder(2.0).with_priority(7.0).build();
        assert_eq!(jump_outline.mode, OutlineMode::JumpFlood);
        assert_eq!(jump_outline.priority, 7.0);
        assert_eq!(jump_outline.overlap, OverlapMode::Merged);

        let switched_outline = MeshOutline::builder(2.0)
            .with_priority(7.0)
            .to_world_hull()
            .to_jump_flood()
            .build();
        assert_eq!(switched_outline.mode, OutlineMode::JumpFlood);
        assert_eq!(switched_outline.priority, 0.0);
        assert_eq!(switched_outline.overlap, OverlapMode::Merged);
    }

    #[test]
    fn legacy_new_api_still_works() {
        let outline = MeshOutline::new(5.0)
            .with_priority(3.0)
            .with_mode(OutlineMode::WorldHull);

        assert_eq!(outline.mode, OutlineMode::WorldHull);
        assert_eq!(outline.width, 5.0);
        assert_eq!(outline.priority, 3.0);
        assert_eq!(outline.overlap, OverlapMode::Merged);
    }
}
