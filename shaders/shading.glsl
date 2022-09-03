// required descriptors:
/*
readonly buffer LightData { PointLight data[]; } lights;
*/

// TODO: Blinn Phong?
float phong(vec3 normal, vec3 vec_to_light, vec3 view_direction,
            float light_intensity, Material mat) {
    float dist_sqd = dot(vec_to_light, vec_to_light);
    float light_radiance = light_intensity / dist_sqd;
    vec3 vec_to_light_norm = vec_to_light * inversesqrt(dist_sqd);

    vec3 light_reflection = reflect(vec_to_light_norm, normal);

    float diffuse = max(dot(vec_to_light_norm, normal), 0.0);
    float specular = 0.0;

    if (diffuse > 0.0) {
        specular =
            pow(max(dot(light_reflection, view_direction), 0.0), mat.shininess);
    }
    // TODO: Tint specular based on metallic parameter
    return light_radiance * (mat.diffuse * diffuse + mat.specular * specular);
}

vec3 shade_object(vec3 direction, HitInfo info, inout vec3 transmission) {
    PointLight light = lights.data[0];  // TODO: multiple lights
    vec3 light_pos = light.position;
    float light_intensity = light.intensity;

    Material mat = info.mat;
    vec3 obj_col = info.colour;

    vec3 vec_to_light = light_pos - info.position;
    vec3 position = info.position;
    vec3 normal = info.normal;

    Ray shadow_ray =
        Ray(position + (normal * EPSILON * 5.0), normalize(vec_to_light));

    float shade = cast_shadow_ray(shadow_ray, vec_to_light);

    // Cheap fresnel
    // TODO: make more physically correct
    float fx = clamp(1.0 - dot(normal, -direction), 0.0, 1.0);
    float fresnel = fx * fx * fx * fx * fx;

    // The amount the object this way was reflected off modifies the colour
    vec3 last_transmission = transmission;
    transmission *=
        clamp(fresnel + mat.reflectivity, 0.0, 1.0) * obj_col;  // obj_colour

    return last_transmission * obj_col *
           (mat.ambient + shade * phong(normal, vec_to_light, direction,
                                        light_intensity, mat));
}
