use specs::{Entities, Join, System, WriteExpect, WriteStorage};

use crate::components::{EntityMoved, MyTurn, Position, Viewshed, WantsToApproach};
use crate::map::Map;

pub struct ApproachAI {}

impl<'a> System<'a> for ApproachAI {
    type SystemData = (
        WriteStorage<'a, MyTurn>,
        WriteStorage<'a, WantsToApproach>,
        WriteStorage<'a, Position>,
        WriteExpect<'a, Map>,
        WriteStorage<'a, Viewshed>,
        WriteStorage<'a, EntityMoved>,
        Entities<'a>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut turns,
            mut want_approach,
            mut positions,
            mut map,
            mut viewsheds,
            mut entity_moved,
            entities,
        ) = data;

        let mut turn_done = vec![];
        for (entity, mut pos, approach, mut viewshed, _myturn) in (
            &entities,
            &mut positions,
            &want_approach,
            &mut viewsheds,
            &turns,
        )
            .join()
        {
            turn_done.push(entity);
            let path = rltk::a_star_search(
                map.xy_idx(pos.x, pos.y) as i32,
                map.xy_idx(approach.idx % map.width, approach.idx / map.width) as i32,
                &mut *map,
            );
            if path.success && path.steps.len() > 1 {
                let mut idx = map.xy_idx(pos.x, pos.y);
                map.blocked[idx] = false;
                pos.x = path.steps[1] as i32 % map.width;
                pos.y = path.steps[1] as i32 / map.width;
                entity_moved
                    .insert(entity, EntityMoved {})
                    .expect("Unable to insert marker");
                idx = map.xy_idx(pos.x, pos.y);
                map.blocked[idx] = true;
                viewshed.dirty = true;
            }
        }

        want_approach.clear();

        for done in turn_done.iter() {
            turns.remove(*done);
        }
    }
}
