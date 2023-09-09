use specs::{Entities, Join, ReadStorage, System, WriteExpect};

use crate::components::Pools;
use crate::{
    components::{BlocksTile, Position},
    map::Map,
    spatial,
};

pub struct MapIndexingSystem {}

impl<'a> System<'a> for MapIndexingSystem {
    type SystemData = (
        WriteExpect<'a, Map>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, BlocksTile>,
        ReadStorage<'a, Pools>,
        Entities<'a>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (map, position, blockers, pools, entities) = data;

        spatial::clear();
        spatial::populate_blocked_from_map(&map);
        for (entity, position) in (&entities, &position).join() {
            let mut alive = true;
            if let Some(pools) = pools.get(entity) {
                if pools.hit_points.current < 1 {
                    alive = false;
                }
            }
            if alive {
                let idx = map.xy_idx(position.x, position.y);
                spatial::index_entity(entity, idx, blockers.get(entity).is_some());
            }
        }
    }
}
