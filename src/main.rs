mod engine;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    println!("Hello World!");
    engine::vk_backend::VkBackend::new("LumenRay", 800, 600);
}
