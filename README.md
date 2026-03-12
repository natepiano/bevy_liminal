# bevy_liminal

[![crates.io](https://img.shields.io/crates/v/bevy_liminal)](https://crates.io/crates/bevy_liminal)
[![CI](https://github.com/natepiano/bevy_liminal/actions/workflows/ci.yml/badge.svg)](https://github.com/natepiano/bevy_liminal/actions/workflows/ci.yml)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/gylleus/bevy_liminal#license)


![Simple outlined cube from example](https://raw.githubusercontent.com/gylleus/bevy_liminal/refs/heads/main/assets/outlined_cube.png)


This plugin provides outline rendering for 3D meshes using a multi-pass GPU approach with JFA *(jump flood algorithm)* for distance field generation.

## Features

- **GPU-optimized rendering** - Uses compute-based jump flood algorithm for efficient and smooth outline generation
- **Customizable outlines** - Control width, color, intensity, and priority per mesh
- **Depth-aware rendering** - Outlines respect depth relationships and handle intersecting geometry
- **HDR support** - Works with both standard and HDR rendering pipelines
- **Animation-friendly** - Supports animated meshes, skinning, and morph targets


## Components

### `MeshOutline`

Add this component to any entity with a `Mesh3d` to enable outline rendering:

```rust
// Basic outline
MeshOutline::new(width)

// Customized outline
MeshOutline::new(10.0)
    .with_color(Color::srgb(1.0, 0.0, 0.0))  // Red outline
    .with_intensity(0.8)                     // 80% strength
    .with_priority(5.0)                      // Higher priority (for overlapping outlines)
```

**Properties:**
- `width: f32` - Outline width in pixels
- `color: Color` - Outline color (supports HDR colors)
- `intensity: f32` - Outline intensity (0.0 to 1.0+)
- `priority: f32` - JumpFlood mode only (higher = front in overlaps)
- `overlap: OverlapMode` - WorldHull/ScreenHull modes only (`Merged` or `Individual`)

Typestate builder (opt-in, non-breaking): `MeshOutline::builder(width) ... .to_world_hull() ... .build()`
Legacy `MeshOutline::new(...)` is backwards compatible and does not enforce mode/property validity at compile time.

### `OutlineCamera`

Mark cameras that should render outlines:


```rust
commands.spawn((
    Camera3d::default(),
    OutlineCamera, // Enable outline rendering for this camera
));
```

`OutlineCamera` automatically requires `DepthPrepass` for proper depth testing.


## Examples

Run the included examples to see the plugin in action:

```bash
# Basic rotating cube with adjustable outline width
cargo run --example simple

# Glowing effect example (HDR)
cargo run --example glowing

# Animated character with outlines
cargo run --example animated_mesh

# Multiple intersecting objects with priority control
cargo run --example intersecting
```

## How It Works

The plugin uses a three-pass GPU rendering approach:

1. **Mask Pass** - Renders outlined meshes to generate seed data and depth information
2. **Jump Flood Algorithm** - Efficiently propagates outline information across the screen using compute shaders
3. **Compose Pass** - Combines the original scene with the computed outline effect by comparing against outline depth

This approach provides:
- Consistent outline width regardless of mesh geometry
- Proper handling of intersecting objects through priority system
- Support for complex mesh features (skinning, morph targets)


## Technical Details

The plugin integrates with Bevy's render graph and adds a custom render node after the main 3D pass. It requires depth prepass to be enabled for proper depth-aware outline rendering.

Outline data is packed into GPU textures using a flood-fill algorithm that efficiently calculates distance fields for smooth, consistent outline rendering across different mesh topologies.


## Versions

| Bevy |    bevy_liminal |
|--------------|--------------|
| 0.18.X       | 0.3.0        |
| 0.17.X       | 0.2.0        |
| 0.16.X       | 0.1.1        |


## Future work

* Improve batching and performance

## License

This repository is free to use and copy and is licensed under either MIT or Apache-2.0.
