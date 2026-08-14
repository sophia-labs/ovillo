# Acknowledgements and shoulders

Ariadne GPU is built on and learns from work by several open-source communities.

## Direct foundations

- **GPUWeb / WebGPU** for the portable compute and rendering model and its
  conformance discipline.
- **gfx-rs `wgpu` and Naga** for a common Rust surface across browser WebGPU,
  Vulkan, Metal, Direct3D, and native validation/translation.
- **WESL and `wesl-rs`** for modular WGSL authoring and build-time compilation.
- **`encase`** for enforcing host-shareable WGSL buffer layouts from Rust.
- **`wgpu-profiler`** for non-blocking timestamp scopes and trace export when
  the adapter exposes timestamp queries.
- **`glam` and `bytemuck`** for compact GPU-compatible vector and event data.

## Architectural influences

- **Vello / Linebender** for compute-centric 2-D rendering and GPU prefix-sum
  techniques.
- **Renderling**, **crabslab**, **craballoc**, and **moongraph** for examples of
  GPU-resident scene/resource organization in Rust.
- The WebGPU CTS for the style of parameterized conformance and negative tests.

No source from these projects is copied in this initial slice. Any later adapted
algorithm or shader must carry a source-level attribution and its required
license notice.
