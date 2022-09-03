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