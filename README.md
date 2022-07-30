# LumenRay
A Real Time Ray Tracing Engine written in rust 

# Running
`cargo run -r`

# Roadmap
* Basic CPU side ray marching (spheres, planes, boxes)
* proper logging system (especially for validation layer messages)
* frames in flight
* materials and textures support
* build out ECS
* support for meshes and loading from .obj
* implement many post processing effects in shaders (depth of field, bloom, material roughness)
* ImGUI
* move ray marching to compute shader
* Scene serialize/deserialize (maybe into yaml/toml)
* physics
* rasterised particles? using z buffer?