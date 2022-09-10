use lumen_ray::{
    engine::Engine,
    renderer::{
        srgb_to_linear, CameraComponent, MaterialComponent, PlaneRenderComponent, PointLightComponent,
        SphereRenderComponent, TransformComponent,
    },
    rgb,
    scene::Scene,
    soft_gray, soft_green, soft_red, soft_yellow, vec2, vec3, white, Vec3,
};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

// TODO: write tests and add codecov
// TODO: add bench with default scene
//TODO: replace unwraps with result propogation (`?`)

fn main() {
    let mut log_builder = env_logger::Builder::new();
    log_builder.filter(None, log::LevelFilter::Debug).init();

    let mut engine = Engine::new(WIDTH, HEIGHT);
    let mut scene = Scene::empty();

    scene.create_entity((
        TransformComponent::with_pos(-1.2, 1.0, 0.1),
        SphereRenderComponent { radius: 1.0 },
        MaterialComponent {
            tex_id: engine.get_texture_by_colour(white!()),
            ambient: 0.05,
            diffuse: 0.03,
            specular: 0.2,
            shininess: 16.0,
            reflectivity: 1.0,
            emissive: 0.0,
            ..Default::default()
        },
    ));
    scene.create_entity((
        TransformComponent::with_pos(1.0, 1.0, -0.7),
        SphereRenderComponent { radius: 1.0 },
        MaterialComponent {
            tex_id: engine.get_texture_by_colour(soft_yellow!()),
            ambient: 0.1,
            diffuse: 1.0,
            specular: 0.9,
            shininess: 32.0,
            reflectivity: 0.25,
            emissive: 0.0,
            ..Default::default()
        },
    ));

    scene.create_entity((
        TransformComponent::with_pos(3.0, 0.0, -8.0),
        PlaneRenderComponent::new(vec3(0.0, 1.0, 0.0)),
        MaterialComponent {
            tex_id: engine.get_texture_by_path("assets/textures/Floor128.bmp"),
            tex_scale: vec2(0.4, 0.4) * vec2(12.0, 12.0),
            ..MaterialComponent::basic()
        },
    ));

    scene.create_entity((
        TransformComponent::with_pos(-3.0, 0.0, -9.0),
        PlaneRenderComponent::new(vec3(1.0, 0.0, 0.0)),
        MaterialComponent {
            tex_id: engine.get_texture_by_colour(soft_red!()),
            ..MaterialComponent::basic()
        },
    ));

    scene.create_entity((
        TransformComponent::with_pos(3.0, 0.0, 3.0),
        PlaneRenderComponent::new(vec3(-1.0, 0.0, 0.0)),
        MaterialComponent {
            tex_id: engine.get_texture_by_colour(soft_green!()),
            ..MaterialComponent::basic()
        },
    ));

    scene.create_entity((
        TransformComponent::with_pos(-3.0, 0.0, 3.0),
        PlaneRenderComponent::new(vec3(0.0, 0.0, -1.0)),
        MaterialComponent {
            tex_id: engine.get_texture_by_colour(soft_gray!()),
            ..MaterialComponent::basic()
        },
    ));
    scene.create_entity((
        TransformComponent::with_pos(3.0, 0.0, -8.0),
        PlaneRenderComponent::new(vec3(0.0, 0.0, 1.0)),
        MaterialComponent {
            tex_id: engine.get_texture_by_colour(soft_gray!()),
            ..MaterialComponent::basic()
        },
    ));

    scene.create_entity((
        TransformComponent::with_pos(3.0, 6.0, 3.0),
        PlaneRenderComponent::new(vec3(0.0, -1.0, 0.0)),
        MaterialComponent {
            tex_id: engine.get_texture_by_colour(soft_gray!()),
            ..MaterialComponent::basic()
        },
    ));

    scene.create_entity((
        TransformComponent::with_pos(2.0, 1.0, 2.5),
        //TransformComponent::with_pos(0.0, 3.0, -2.5),
        PointLightComponent { intensity: 5.0 },
    ));

    scene.create_entity((
        TransformComponent::with_pos(0.0, 3.5, -8.5),
        CameraComponent {
            pitch: 0.0,
            yaw:   0.0,
            fov:   90.0,
        },
    ));

    engine.run(scene);
}
