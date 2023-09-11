use crate::astar::a_star_search;
use rltk::RandomNumberGenerator;
use specs::{Entities, Join, ReadExpect, ReadStorage, System, WriteExpect, WriteStorage};

use crate::components::{ApplyMove, MoveMode, Movement, MyTurn, Position};
use crate::map::Map;

pub struct DefaultMoveAI {}

impl<'a> System<'a> for DefaultMoveAI {
    type SystemData = (
        WriteStorage<'a, MyTurn>,
        WriteStorage<'a, MoveMode>,
        ReadStorage<'a, Position>,
        ReadExpect<'a, Map>,
        WriteExpect<'a, RandomNumberGenerator>,
        Entities<'a>,
        WriteStorage<'a, ApplyMove>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut turns, mut move_mode, positions, map, mut rng, entities, mut apply_move) = data;

        let mut turn_done = vec![];
        for (entity, pos, mode, _myturn) in (&entities, &positions, &mut move_mode, &turns).join() {
            turn_done.push(entity);

            match &mut mode.mode {
                Movement::Static => {}
                Movement::RandomWaypoint { path } => {
                    if let Some(path) = path {
                        if path.len() > 1 {
                            if !crate::spatial::is_blocked(path[1]) {
                                apply_move
                                    .insert(entity, ApplyMove { dest_idx: path[1] })
                                    .expect("Unable to insert");
                                path.remove(0);
                                turn_done.push(entity);
                            }
                        } else {
                            mode.mode = Movement::RandomWaypoint { path: None };
                        }
                    } else {
                        let target_x = rng.roll_dice(1, map.width - 2);
                        let target_y = rng.roll_dice(1, map.height - 2);
                        let idx = map.xy_idx(target_x, target_y);
                        if map.tiles[idx].is_walkable() {
                            let path = a_star_search(
                                map.xy_idx(pos.x, pos.y),
                                map.xy_idx(target_x, target_y),
                                &*map,
                            );
                            if path.success && path.steps.len() > 1 {
                                mode.mode = Movement::RandomWaypoint {
                                    path: Some(path.steps),
                                };
                            }
                        }
                    }
                }
                Movement::Random => {
                    let mut x = pos.x;
                    let mut y = pos.x;
                    let move_roll = rng.roll_dice(1, 5);
                    match move_roll {
                        1 => x -= 1,
                        2 => x += 1,
                        3 => y -= 1,
                        4 => y += 1,
                        _ => {}
                    }

                    if x > 0 && x < map.width - 1 && y > 0 && y < map.height - 1 {
                        let dest_idx = map.xy_idx(x, y);
                        if !crate::spatial::is_blocked(dest_idx) {
                            apply_move
                                .insert(entity, ApplyMove { dest_idx })
                                .expect("Unable to insert");
                            turn_done.push(entity);
                        }
                    }
                }
            }
        }

        for done in &turn_done {
            turns.remove(*done);
        }
    }
}
