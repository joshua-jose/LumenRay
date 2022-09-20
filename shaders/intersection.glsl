// readonly buffer SphereData { Sphere data[]; } spheres;
// readonly buffer PlaneData { Plane data[]; } planes;
// uniform texture2D textures[];

float ray_sphere_intersect(Ray ray, Sphere sphere) {
    // TODO: reduce the amount of "if" statements in here
    //  vector from sphere center to ray origin.
    vec3 c_to_o = ray.origin - sphere.position;

    // quadratic formula constants for line-sphere intersection
    // TODO: precompute r^2?
    float a = dot(ray.direction, ray.direction);
    float b = 2.0 * dot(ray.direction, c_to_o);
    float c = dot(c_to_o, c_to_o) - (sphere.radius * sphere.radius);

    float d0 = FLT_MAX, d1 = FLT_MAX;

    // distance of two intersection points
    float discrim = (b * b) - (4.0 * a * c);

    if (discrim >= 0.0) {
        // now solve the quadratic, using a more stable computer friendly
        // formula

        // if the discrim is close to 0, use a faster formula ignoring
        // discrim. This can be optimised by being looser on what is "close"
        // to zero.
        /*
        if (abs(discrim) <= EPSILON) {
            d0 = -0.5 * b / a;
            d1 = d0;
        } else {
            */
        float sqrt_discrim = sqrt(discrim);
        float q = -0.5 * (b + (sign(b) * sqrt_discrim));
        d0 = q / a;
        d1 = c / q;
        //}

        /* TODO:
        if (d0 < 0.0) d0 = FLT_MAX;
        if (d1 < 0.0) d1 = FLT_MAX;

        d0 = min(d0, d1);
        */

        if (d0 > d1) {
            float temp = d0;
            d0 = d1;
            d1 = temp;
        };

        // negative distances mean we intersect behind, we want d0 to be the
        // positive intersection
        if (d0 < 0.0) {
            d0 = d1;
            if (d0 < 0.0) {
                d0 = FLT_MAX;
            }
        }
    }
    return d0;
}

float ray_plane_intersect(Ray ray, Plane plane) {
    float denom = -dot(plane.normal, ray.direction);

    if (denom > 1e-6) {
        vec3 to_plane = plane.position - ray.origin;
        float d = dot(to_plane, -plane.normal) / denom;

        if (d >= 0.0) {
            vec3 tangent = plane.tangent;
            vec3 bitangent = cross(plane.normal, tangent);
            vec3 position = ray.origin + (d * ray.direction);

            vec3 delta = position - plane.position;
            vec2 xy = vec2(0.5 * plane.width, 0.5 * plane.height) +
                      vec2(dot(tangent, delta), dot(bitangent, delta));

            if (xy.x > plane.width || xy.x < 0.0 || xy.y > plane.height ||
                xy.y < 0.0) {
                return FLT_MAX;
            } else {
                return d;
            }
        } else {
            return FLT_MAX;
        }

    } else {
        return FLT_MAX;
    }
}

float ray_triangle_intersect(Ray ray, Vertex v1, Vertex v2, Vertex v3,
                             inout float u, inout float v) {
    vec3 edge1 = v2.position - v1.position;
    vec3 edge2 = v3.position - v1.position;

    vec3 h = cross(ray.direction, edge2);
    float det = dot(h, edge1);

    // if (det < 0.0) return FLT_MAX;  // Back faces culled
    //  if (abs(det) < EPSILON) return FLT_MAX; // parallel

    float invDet = 1.0 / det;
    vec3 s = ray.origin - v1.position;
    u = invDet * dot(s, h);

    if (u < 0.0 || u > 1.0) return FLT_MAX;  // outside triangle

    vec3 q = cross(s, edge1);
    v = invDet * dot(ray.direction, q);

    if (v < 0.0 || (u + v) > 1.0) return FLT_MAX;

    float t = invDet * dot(edge2, q);

    if (t < 0.0) return FLT_MAX;  // intersection behind
    return t;
}

HitInfo cast_ray(Ray ray) {
    // Expects SphereData, PlaneData in scope
    float least_dist = FLT_MAX;
    uint hit_idx = UINT_MAX;
    uint hit_obj = UINT_MAX;
    uint triangle_idx = UINT_MAX;
    vec2 triangle_uv = vec2(FLT_MAX);

    for (int i = 0; i < spheres.length(); i++) {
        float obj_dist = ray_sphere_intersect(ray, spheres[i]);
        if (obj_dist < least_dist) {
            least_dist = obj_dist;
            hit_idx = i;
            hit_obj = 0;  // TODO: replace with var
        }
    }

    for (int i = 0; i < planes.length(); i++) {
        float obj_dist = ray_plane_intersect(ray, planes[i]);
        if (obj_dist < least_dist) {
            least_dist = obj_dist;
            hit_idx = i;
            hit_obj = 1;  // TODO: replace with var
        }
    }

    // Go through mesh instances
    for (int i = 0; i < mesh_instances.length(); i++) {
        MeshInstance m = mesh_instances[i];

        // Go through all triangles of that mesh
        for (uint t = m.start_triangle_idx;
             t < (m.start_triangle_idx + m.num_triangles); t++) {
            Triangle triangle = triangles[t];

            Vertex v1 = vertices[m.start_vertex_idx + triangle.v1_idx];
            Vertex v2 = vertices[m.start_vertex_idx + triangle.v2_idx];
            Vertex v3 = vertices[m.start_vertex_idx + triangle.v3_idx];

            // translate vertices
            v1.position += m.position;
            v2.position += m.position;
            v3.position += m.position;

            float tu;
            float tv;
            float obj_dist = ray_triangle_intersect(ray, v1, v2, v3, tu, tv);

            // Show vertices
            /* Sphere sphere =
                Sphere(v1.position, 0.15,
                       Material(0, vec2(1.0), 1.0, 1.0, 0.0, 0.0, 0.0,
            0.0)); obj_dist = min(obj_dist, ray_sphere_intersect(ray,
            sphere));

            sphere =
                Sphere(v2.position, 0.15,
                       Material(0, vec2(1.0), 1.0, 1.0, 0.0, 0.0, 0.0,
            0.0)); obj_dist = min(obj_dist, ray_sphere_intersect(ray,
            sphere)); sphere = Sphere(v3.position, 0.15, Material(0,
            vec2(1.0), 1.0, 1.0, 0.0, 0.0, 0.0, 0.0)); obj_dist =
            min(obj_dist, ray_sphere_intersect(ray, sphere)); */

            if (obj_dist < least_dist) {
                least_dist = obj_dist;
                hit_idx = i;
                triangle_idx = t;
                hit_obj = 2;  // TODO: replace with var
                triangle_uv = vec2(tu, tv);
            }
        }
    }

    if (least_dist < FLT_MAX) {
        vec3 position = ray.origin + (least_dist * ray.direction);
        vec3 normal;
        Material mat;
        vec2 uv;

        if (hit_obj == 0) {
            Sphere sphere = spheres[hit_idx];
            normal = normalize(position - sphere.position);
            mat = sphere.mat;

            uv = vec2(0.5 + (atan(normal.x, -normal.z) / TAU),
                      0.5 + (asin(normal.y) / PI));

        } else if (hit_obj == 1) {
            Plane plane = planes[hit_idx];
            mat = plane.mat;
            normal = plane.normal;

            vec3 tangent = plane.tangent;
            vec3 bitangent = cross(normal, tangent);

            vec3 delta = position - plane.position;
            uv = vec2(0.5) + vec2(dot(tangent, delta) / plane.width,
                                  dot(bitangent, delta) / plane.height);
        } else if (hit_obj == 2) {
            MeshInstance m = mesh_instances[hit_idx];
            Triangle triangle = triangles[triangle_idx];

            Vertex v1 = vertices[m.start_vertex_idx + triangle.v1_idx];
            Vertex v2 = vertices[m.start_vertex_idx + triangle.v2_idx];
            Vertex v3 = vertices[m.start_vertex_idx + triangle.v3_idx];

            // translate vertices
            v1.position += m.position;
            v2.position += m.position;
            v3.position += m.position;

            float r = triangle_uv.x;
            float s = triangle_uv.y;
            float w = 1.0 - r - s;

            normal =
                normalize((w * v1.normal) + (r * v2.normal) + (s * v3.normal));
            // normal = normalize(v1.normal + v2.normal + v3.normal);
            mat = m.mat;

            uv = (w * v1.uv) + (r * v2.uv) + (s * v3.uv);
            // uv = vec2(0.5);
        }

        uint lm_idx = hit_idx;
        if (hit_obj > 0) {
            lm_idx += spheres.length();
        }
        if (hit_obj > 1) {
            lm_idx += planes.length();
        }

        vec3 colour = sample_texture(mat, uv);
        vec3 radiosity = sample_lightmap(lm_idx, uv);

        return HitInfo(position, normal, mat, colour, radiosity);

    } else {
        return HitInfo(vec3(FLT_MAX), vec3(FLT_MAX), NULL_MAT, vec3(FLT_MAX),
                       vec3(FLT_MAX));
    }
}

// TODO: This technically causes a "penumbra" cast on objects by themselves.
// Not sure if thats correct? Should that happen *on top* of lambertian
// attenuation? mostly noticeable in radiosity and needs paying attention to
float cast_shadow_ray(Ray ray, vec3 vec_to_light) {
    // Expects SphereData, PlaneData in scope
    // ray must be normalized
    // ignores planes as they dont cast shadows
    // uses distance between a point and a line to find the closest approach
    // between the ray and a sphere
    float light_dist = length(vec_to_light);
    // float least_dist = FLT_MAX;

    const float SHADING_K = 16.0;  // TODO: make actual light size
    float shade = 1.0;

    for (int i = 0; i < spheres.length(); i++) {
        Sphere sphere = spheres[i];
        float obj_dist = ray_sphere_intersect(ray, sphere);

        vec3 p = sphere.position;
        vec3 a = ray.origin;
        vec3 d = p - a;

        float projected_length = dot(d, ray.direction);

        if (projected_length > light_dist || projected_length < 0.0) continue;

        float closest_approach =
            length(d - (projected_length * ray.direction)) - sphere.radius;
        closest_approach = max(closest_approach, 0.0);

        shade = min(
            shade, smoothstep(0.0, 1.0,
                              SHADING_K * closest_approach / projected_length));

        // least_dist = min(least_dist, obj_dist);
    }

    for (int i = 0; i < mesh_instances.length(); i++) {
        MeshInstance m = mesh_instances[i];

        // Go through all triangles of that mesh
        for (uint t = m.start_triangle_idx;
             t < (m.start_triangle_idx + m.num_triangles); t++) {
            Triangle triangle = triangles[t];

            Vertex v1 = vertices[m.start_vertex_idx + triangle.v1_idx];
            Vertex v2 = vertices[m.start_vertex_idx + triangle.v2_idx];
            Vertex v3 = vertices[m.start_vertex_idx + triangle.v3_idx];

            // translate vertices
            v1.position += m.position;
            v2.position += m.position;
            v3.position += m.position;

            float tu;
            float tv;
            float projected_length =
                ray_triangle_intersect(ray, v1, v2, v3, tu, tv);

            // we directly hit the triangle and it's blocking light
            if (projected_length < light_dist && projected_length > 0.0) {
                shade = min(shade, 0.0);
            } else {
                vec3 vp1 = v1.position;
                vec3 vp2 = v2.position;
                vec3 vp3 = v3.position;

                vec3 edges[3][2] = {{vp1, vp2}, {vp1, vp3}, {vp2, vp3}};

                for (int e = 0; e < 3; e++) {
                    vec3 vertices[2] = edges[e];
                    vec3 seg_dir = vertices[1] - vertices[0];

                    vec3 p1 = ray.origin;
                    vec3 p2 = (ray.origin + ray.direction);
                    vec3 p3 = vertices[0];
                    vec3 p4 = vertices[1];

                    vec3 V1 = p2 - p1;
                    vec3 V2 = p4 - p3;
                    vec3 V21 = p3 - p1;

                    float v11 = dot(V1, V1);
                    float v21 = dot(V2, V1);
                    float v22 = dot(V2, V2);
                    float v21_2 = dot(V21, V2);
                    float v21_1 = dot(V21, V1);
                    float denom = v21 * v21 - v22 * v11;

                    float s;
                    float t;

                    /* if (abs(denom) < EPSILON) {
                        s = 0;
                        t = (v11 * s - v21_1) / v21;
                    } else { */
                    s = (v21_2 * v21 - v22 * v21_1) / denom;
                    t = (-v21_1 * v21 + v11 * v21_2) / denom;
                    /* } */

                    if (s < 0.0 || s > light_dist) continue;
                    t = clamp(t, 0.0, 1.0);

                    vec3 closest_point_ray = p1 + s * V1;
                    vec3 closest_point_edge = p3 + t * V2;

                    float projected_length =
                        abs(length(closest_point_ray - ray.origin));

                    float closest_approach =
                        abs(length(closest_point_ray - closest_point_edge));

                    shade = min(shade, smoothstep(0.0, 1.0,
                                                  SHADING_K * closest_approach /
                                                      projected_length));
                }
            }

            // least_dist = min(least_dist, obj_dist);
        }
    }

    return shade;
};