use rltk::DijkstraMap;
use specs::{Entities, Join, System, WriteExpect, WriteStorage};

use crate::components::{EntityMoved, MyTurn, Position, Viewshed, WantsToFlee};
use crate::map::Map;

pub struct FleeAI {}

impl<'a> System<'a> for FleeAI {
    type SystemData = (
        WriteStorage<'a, MyTurn>,
        WriteStorage<'a, WantsToFlee>,
        WriteStorage<'a, Position>,
        WriteExpect<'a, Map>,
        WriteStorage<'a, Viewshed>,
        WriteStorage<'a, EntityMoved>,
        Entities<'a>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut turns,
            mut want_flee,
            mut positons,
            mut map,
            mut viewsheds,
            mut entity_moved,
            entities,
        ) = data;

        let mut turn_done = vec![];
        for (entity, mut pos, flee, mut viewshed, _myturn) in
            (&entities, &mut positons, &want_flee, &mut viewsheds, &turns).join()
        {
            turn_done.push(entity);
            let my_idx = map.xy_idx(pos.x, pos.y);
            map.populate_blocked();
            let flee_map = DijkstraMap::new(
                map.width as usize,
                map.height as usize,
                &flee.indices,
                &*map,
                100.0,
            );
            let flee_target = DijkstraMap::find_highest_exit(&flee_map, my_idx, &*map);
            if let Some(flee_target) = flee_target {
                if !map.blocked[flee_target] {
                    map.blocked[my_idx] = false;
                    map.blocked[flee_target] = true;
                    viewshed.dirty = true;
                    pos.x = flee_target as i32 % map.width;
                    pos.y = flee_target as i32 / map.width;
                    entity_moved
                        .insert(entity, EntityMoved {})
                        .expect("Unable to insert marker");
                }
            }
        }
        want_flee.clear();

        for done in turn_done.iter() {
            turns.remove(*done);
        }
    }
}
