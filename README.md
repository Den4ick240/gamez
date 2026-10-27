# High-Performance GPU Simulation Engine (Rust & wgpu)

An advanced real-time physics and particle simulation engine built from scratch in **Rust**, leveraging **wgpu** and custom WGSL compute shaders for GPU-accelerated parallel execution.

## 🚀 Key Features

* **GPU Compute Pipelines:** Custom WGSL compute shaders handling heavy particle and physics loops directly on the GPU.
* **Spatial Partitioning:** High-performance spatial hashing structures (`pointer_hash`, `sorting_hash`, `fixed_size_grid`) optimized for broad-phase collision and neighbor searches.
* **Custom Integrators & Constraints:** Rigid body and particle integration pipelines with custom box boundaries and constraint solvers.
* **Real-time Profiling & Hot-Reloading:** Includes custom debugging profilers and shader file watchers for rapid iteration.

## 🛠️ Tech Stack & Concepts

* **Language:** Rust (Safe concurrency, zero-cost abstractions)
* **Graphics & Compute API:** `wgpu` (Cross-platform modern GPU API)
* **Shaders:** WGSL (WebGPU Shading Language)
* **Architecture:** Entity-less spatial simulation, compute-driven particle pipelines
