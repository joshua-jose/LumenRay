# LumenRay
A Real Time Ray Tracing Engine written in rust 

# Running
`cargo run -r`

# Roadmap
* Basic CPU side ray marching (spheres, planes, boxes)
* frames in flight
* materials and textures support
* build out ECS
* support for meshes and loading from .obj
* implement many post processing effects in shaders (depth of field, bloom, material roughness, fog, volumetric lighting, glare)
* ImGUI
* move ray marching to compute shader
* different types of renderers to support a compute based ray engine, CPU based, and vk ray tracing pipeline based (ComputeRenderer,CPUStreamingRenderer,AcceleratedRenderer)
* Scene serialize/deserialize (maybe into yaml/toml)
* physics
* rasterised particles? using z buffer?