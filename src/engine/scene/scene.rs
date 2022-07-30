// scene module

// implement scene.create_entity()
// scene.query_mut
// scene.prepared_query
// scene.prepared_view

/*
struct Entity {
    id: hecs::Entity,
    scene: Weak<RefCell<Scene>> // gets a weak pointer, so we don't take ownership of the scene
}

impl Entity{
    fn add_component(component: hecs::Component){
        self.scene.add_component(self.id, component);
    }
    fn add_components(){};
    fn remove_component(){};
}
*/
