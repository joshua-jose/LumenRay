// scene module

// implement scene.create_entity()
// scene.query_mut
// scene.prepared_query
// scene.prepared_view

use super::{Entity, NoSuchEntity, RenderScene};
use hecs::{DynamicBundle, World};
use std::{cell::RefCell, error::Error, fmt, sync::Arc};

pub struct Scene {
    pub(super) world: Arc<RefCell<World>>,
}

/*  TODO: figure out the performance impact of all these wrappers,
    especially for code that will run every game loop (querying and viewing related code),
    the amount of borrow checking and ref counting should be kept down to like once per game loop
*/

impl Scene {
    pub fn empty() -> Self {
        Self {
            world: Arc::new(RefCell::new(World::new())),
        }
    }
    pub fn create_entity(&mut self, components: impl DynamicBundle) -> Entity {
        let hecs_entity = self.world.borrow_mut().spawn(components);
        Entity {
            id:    hecs_entity,
            world: Arc::downgrade(&self.world),
        }
    }
    pub unsafe fn destroy_entity_by_id(&mut self, id: u32) -> Result<(), NoSuchEntity> {
        //! # Safety
        //! make sure the entity with that id exists
        let mut world = self.world.borrow_mut();
        let entity = world.find_entity_from_id(id);

        match world.despawn(entity) {
            Ok(_) => Ok(()),
            Err(_) => Err(NoSuchEntity),
        }
    }

    pub unsafe fn add_component_by_id(&mut self, id: u32, components: impl DynamicBundle) -> Result<(), NoSuchEntity> {
        //! # Safety
        //! make sure the entity with that id exists
        let mut world = self.world.borrow_mut();
        let entity = world.find_entity_from_id(id);

        match world.insert(entity, components) {
            Ok(_) => Ok(()),
            Err(_) => Err(NoSuchEntity),
        }
    }

    pub fn query<Q: hecs::Query>(&self) -> hecs::QueryBorrow<Q> {
        let ptr = self.world.as_ptr();

        unsafe { (*ptr).query::<Q>() }
    }

    pub(super) fn query_owned<Q: hecs::Query>(
        &self,
    ) -> Vec<(hecs::Entity, <<Q as hecs::Query>::Fetch as hecs::Fetch<'_>>::Item)> {
        let ptr = self.world.as_ptr();

        unsafe { (*ptr).query_mut::<Q>().into_iter().collect::<Vec<_>>() }
    }

    /*
    fn entity_from_id(&mut self, hecs_entity: hecs::Entity) -> Entity {
        Entity {
            id:    hecs_entity,
            world: Arc::downgrade(&self.world),
        }
    }
    */

    pub fn query_scene_objects(&mut self) -> RenderScene {
        //! Query won't be valid if Scene is destroyed
        RenderScene::from_scene(self)
    }
}

impl Default for Scene {
    fn default() -> Self { Self::empty() }
}

/// Error indicating that a scene has been destroyed
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NoSuchScene;

impl fmt::Display for NoSuchScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.pad("no such scene") }
}

impl Error for NoSuchScene {}

pub type SceneResult<T> = Result<T, NoSuchScene>;
