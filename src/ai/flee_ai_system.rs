use rltk::DijkstraMap;
use specs::{Entities, Join, ReadStorage, System, WriteExpect, WriteStorage};

use crate::components::{ApplyMove, MyTurn, Position, WantsToFlee};
use crate::map::Map;

pub struct FleeAI {}

impl<'a> System<'a> for FleeAI {
    type SystemData = (
        WriteStorage<'a, MyTurn>,
        WriteStorage<'a, WantsToFlee>,
        ReadStorage<'a, Position>,
        WriteExpect<'a, Map>,
        Entities<'a>,
        WriteStorage<'a, ApplyMove>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut turns, mut want_flee, positions, mut map, entities, mut apply_move) = data;

        let mut turn_done = vec![];
        for (entity, pos, flee, _myturn) in (&entities, &positions, &want_flee, &turns).join() {
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
                if !crate::spatial::is_blocked(flee_target) {
                    apply_move
                        .insert(
                            entity,
                            ApplyMove {
                                dest_idx: flee_target,
                            },
                        )
                        .expect("Unable to insert");
                    turn_done.push(entity);
                }
            }
        }
        want_flee.clear();

        for done in turn_done.iter() {
            turns.remove(*done);
        }
    }
}
