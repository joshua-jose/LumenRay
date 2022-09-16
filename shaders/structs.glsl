// Data Structures
struct Ray {
    vec3 origin;
    vec3 direction;
};

struct Material {
    uint tex_id;
    vec2 tex_scale;

    float ambient;
    float diffuse;
    float specular;
    float shininess;
    float reflectivity;
    float emissive;
};

struct HitInfo {
    vec3 position;
    vec3 normal;
    Material mat;
    vec3 colour;
    vec3 radiosity;
};

struct Sphere {
    vec3 position;
    float radius;
    Material mat;
};

struct Plane {
    vec3 position;
    vec3 normal;
    vec3 tangent;
    float width;
    float height;
    Material mat;
};

struct Vertex {
    vec3 position;
    vec3 normal;
    vec2 uv;
};

struct Triangle {
    uint v1_idx;
    uint v2_idx;
    uint v3_idx;
};

struct MeshInstance {
    vec3 position;
    uint start_triangle_idx;
    uint start_vertex_idx;
    uint num_triangles;
    Material mat;
};

struct PointLight {
    vec3 position;
    float intensity;
};

struct Camera {
    vec3 position;
    mat3 rotation;
    float zdepth;
};

const Material NULL_MAT =
    Material(0, vec2(0.0, 0.0), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);