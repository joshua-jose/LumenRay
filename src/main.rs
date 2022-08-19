use lumen_ray::{
    engine::Engine,
    renderer::{
        CameraComponent, MaterialComponent, PlaneRenderComponent, PointLightComponent, SphereRenderComponent,
        TransformComponent, SOFT_GREEN, WHITE,
    },
    scene::Scene,
    vec3,
};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

// TODO: move all this to engine class

// TODO: write tests and add codecov
// TODO: add bench with default scene

// TODO: audit game loop for unnecessary performance hits

fn main() {
    let mut log_builder = env_logger::Builder::new();
    log_builder.filter(None, log::LevelFilter::Debug).init();

    let mut scene = Scene::empty();

    scene.create_entity((
        TransformComponent::with_pos(0.0, 0.0, 3.0),
        SphereRenderComponent { radius: 1.0 },
        MaterialComponent::basic(),
    ));
    scene.create_entity((
        TransformComponent::with_pos(-2.0, 0.0, 3.0),
        SphereRenderComponent { radius: 1.0 },
        MaterialComponent {
            colour:       WHITE,
            ambient:      0.05,
            diffuse:      0.03,
            specular:     0.2,
            shininess:    16.0,
            reflectivity: 1.0,
            emissive:     0.0,
        },
    ));
    scene.create_entity((
        TransformComponent::with_pos(2.0, 0.0, 3.0),
        SphereRenderComponent { radius: 1.0 },
        MaterialComponent {
            colour:       WHITE,
            ambient:      0.05,
            diffuse:      0.03,
            specular:     0.2,
            shininess:    16.0,
            reflectivity: 1.0,
            emissive:     0.0,
        },
    ));
    scene.create_entity((
        TransformComponent::with_pos(0.0, 2.0, 3.0),
        SphereRenderComponent { radius: 1.0 },
        MaterialComponent::basic(),
    ));

    scene.create_entity((
        TransformComponent::with_pos(0.0, -1.0, 0.0),
        PlaneRenderComponent {
            normal: vec3(0.0, 1.0, 0.0),
        },
        MaterialComponent {
            colour: SOFT_GREEN,
            ..MaterialComponent::basic()
        },
    ));

    scene.create_entity((
        TransformComponent::with_pos(0.0, 2.0, -1.0),
        PointLightComponent { intensity: 10.0 },
    ));

    scene.create_entity((
        TransformComponent::with_pos(0.0, 0.0, -5.0),
        CameraComponent {
            pitch: 0.0,
            yaw:   0.0,
            fov:   90.0,
        },
    ));

    let engine = Engine::new(WIDTH, HEIGHT, scene);
    engine.run();
}
