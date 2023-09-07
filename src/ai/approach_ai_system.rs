use specs::{Entities, Join, ReadStorage, System, WriteExpect, WriteStorage};

use crate::components::{EntityMoved, MyTurn, Name, Position, Viewshed, WantsToApproach};
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
        ReadStorage<'a, Name>,
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
            names,
        ) = data;

        let mut turn_done = vec![];
        for (entity, mut pos, approach, mut viewshed, _myturn, name) in (
            &entities,
            &mut positions,
            &want_approach,
            &mut viewsheds,
            &turns,
            &names,
        )
            .join()
        {
            turn_done.push(entity);
            let path = rltk::a_star_search(
                map.xy_idx(pos.x, pos.y) as i32,
                map.xy_idx(approach.idx % map.width, approach.idx / map.width) as i32,
                &*map,
            );
            if path.success && path.steps.len() > 1 {
                let idx = map.xy_idx(pos.x, pos.y);
                pos.x = path.steps[1] as i32 % map.width;
                pos.y = path.steps[1] as i32 / map.width;
                entity_moved
                    .insert(entity, EntityMoved {})
                    .expect("Unable to insert marker");
                let new_idx = map.xy_idx(pos.x, pos.y);
                crate::spatial::move_entity(entity, idx, new_idx);
                viewshed.dirty = true;
            }
        }

        want_approach.clear();

        for done in turn_done.iter() {
            turns.remove(*done);
        }
    }
}
