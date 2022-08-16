use std::{cell::RefCell, error::Error, fmt, sync::Weak};

use hecs::{Component, DynamicBundle, World};

use super::{NoSuchScene, SceneResult};

pub struct Entity {
    pub(crate) id:    hecs::Entity,
    pub(crate) world: Weak<RefCell<World>>, // gets a weak pointer, so we don't take ownership of the scene
}

impl Entity {
    pub fn new(id: hecs::Entity, world: Weak<RefCell<World>>) -> Self { Self { id, world } }

    pub fn add_components(&mut self, components: impl DynamicBundle) -> SceneResult<()> {
        let world = self.world.upgrade().ok_or(NoSuchScene)?;
        world.borrow_mut().insert(self.id, components).unwrap();

        Ok(())
    }
    pub fn add_component(&mut self, component: impl Component) -> SceneResult<()> {
        let world = self.world.upgrade().ok_or(NoSuchScene)?;
        world.borrow_mut().insert_one(self.id, component).unwrap();
        Ok(())
    }
    pub fn remove_component<C: Component>(&mut self) -> SceneResult<()> {
        let world = self.world.upgrade().ok_or(NoSuchScene)?;
        world.borrow_mut().remove_one::<C>(self.id).unwrap();
        Ok(())
    }

    pub fn destroy(&mut self) -> SceneResult<()> {
        let world = self.world.upgrade().ok_or(NoSuchScene)?;
        world.borrow_mut().despawn(self.id).expect("This entity doesn't exist");
        Ok(())
    }

    pub fn get_id(&self) -> u32 { self.id.id() }
}

/// Error indicating that no entity with a particular ID exists
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NoSuchEntity;

impl fmt::Display for NoSuchEntity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.pad("no such entity") }
}

impl Error for NoSuchEntity {}

impl From<hecs::NoSuchEntity> for NoSuchEntity {
    fn from(_: hecs::NoSuchEntity) -> Self { NoSuchEntity }
}
