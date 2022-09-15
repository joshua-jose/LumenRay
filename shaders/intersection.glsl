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

HitInfo cast_ray(Ray ray) {
    // Expects SphereData, PlaneData in scope
    float least_dist = FLT_MAX;
    uint hit_idx = UINT_MAX;
    uint hit_obj = UINT_MAX;

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
        }

        uint lm_idx = (hit_obj * spheres.length()) + hit_idx;

        vec3 colour = sample_texture(mat, uv);
        vec3 radiosity = sample_lightmap(lm_idx, uv);

        return HitInfo(position, normal, mat, colour, radiosity);

    } else {
        return HitInfo(vec3(FLT_MAX), vec3(FLT_MAX), NULL_MAT, vec3(FLT_MAX),
                       vec3(FLT_MAX));
    }
}

// TODO: This technically causes a "penumbra" cast on objects by themselves. Not
// sure if thats correct? Should that happen *on top* of lambertian attenuation?
// mostly noticeable in radiosity and needs paying attention to
float cast_shadow_ray(Ray ray, vec3 vec_to_light) {
    // Expects SphereData, PlaneData in scope
    // ray must be normalized
    // ignores planes as they dont cast shadows
    // uses distance between a point and a line to find the closest approach
    // between the ray and a sphere
    float light_dist = length(vec_to_light);
    float least_dist = FLT_MAX;

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

        least_dist = min(least_dist, obj_dist);
    }

    return shade;
};