use crate::renderer::{SphereRenderComponent, TransformComponent};
use hecs::{PreparedQuery, PreparedQueryIter, Query as HecsQuery};

use super::{NoSuchScene, Scene, SceneResult};

#[derive(HecsQuery)]
pub struct SphereRenderQuery<'a> {
    pub transform: &'a TransformComponent,
    pub render:    &'a SphereRenderComponent,
}

pub struct Query<Q: HecsQuery> {
    query: PreparedQuery<Q>,
}

// TODO: The iterator this returns *should* return a lumenray entity, not a hecs one
impl<Q: HecsQuery> Query<Q> {
    pub fn new() -> Self {
        let query = PreparedQuery::<Q>::new();
        Self { query }
    }

    //pub fn view(&mut self) -> hecs::PreparedView<Q> { self.query.view_mut(&mut self.world) }
    pub fn query(&mut self, scene: &mut Scene) -> SceneResult<PreparedQueryIter<'_, Q>> {
        let world = &scene.world;

        // TODO: return an iter than holds a strong ref to world?
        /*  Safety:
            This could fall apart if world gets destroyed while the iter is still alive.
        */
        let ptr = world.as_ptr();

        unsafe { Ok(self.query.query_mut(ptr.as_mut().ok_or(NoSuchScene)?)) }
    }
}

impl<Q: HecsQuery> Default for Query<Q> {
    fn default() -> Self { Self::new() }
}
