# LumenRay
A Real Time Ray Tracing Engine written in rust 

# Running
`cargo run -r`

# Roadmap
* radiosity
* Use a BVH (AABB) for ray traversal
* support for meshes and loading from .obj or using assimp
* implement many post processing effects in shaders (depth of field, bloom, material roughness, fog, volumetric lighting, glare)
* PBR
* refraction
* photon mapping?
* ImGUI
* different types of renderers to support a compute based ray engine, CPU based, and vk ray tracing pipeline based (ComputeRenderer,CPUStreamingRenderer,AcceleratedRenderer)
* Scene serialize/deserialize (maybe into yaml/toml)
* physics
* rasterised particles? using z buffer?