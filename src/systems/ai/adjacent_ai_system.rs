use specs::{Entities, Entity, Join, ReadExpect, ReadStorage, System, WriteStorage};

use crate::components::{Faction, MyTurn, Position, TileSize, WantsToMelee};
use crate::map::Map;
use crate::raws::rawmaster::{faction_reaction, RAWS};
use crate::raws::Reaction;
use crate::rect::Rect;
use crate::spatial;

pub struct AdjacentAI {}

impl<'a> System<'a> for AdjacentAI {
    type SystemData = (
        WriteStorage<'a, MyTurn>,
        ReadStorage<'a, Faction>,
        ReadStorage<'a, Position>,
        ReadExpect<'a, Map>,
        WriteStorage<'a, WantsToMelee>,
        Entities<'a>,
        ReadExpect<'a, Entity>,
        ReadStorage<'a, TileSize>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut turns, factions, positions, map, mut want_melee, entities, player, sizes) = data;

        let mut turn_done = vec![];
        for (entity, _turn, my_faction, pos) in (&entities, &turns, &factions, &positions).join() {
            if entity == *player {
                continue;
            }

            let mut reactions = vec![];
            let idx = map.xy_idx(pos.x, pos.y);
            let w = map.width;
            let h = map.height;

            if let Some(size) = sizes.get(entity) {
                let mob_rect = Rect::new(pos.x, pos.y, size.x, size.y).get_all_tiles();
                let parent_rect = Rect::new(pos.x - 1, pos.y - 1, size.x + 2, size.y + 2);
                parent_rect
                    .get_all_tiles()
                    .iter()
                    .filter(|t| !mob_rect.contains(t))
                    .for_each(|t| {
                        if t.0 > 0 && t.0 < w - 1 && t.1 > 0 && t.1 < h - 1 {
                            let target_idx = map.xy_idx(t.0, t.1);
                            evaluate(
                                target_idx,
                                &map,
                                &factions,
                                &my_faction.name,
                                &mut reactions,
                            );
                        }
                    });
            } else {
                if pos.x > 0 {
                    evaluate(idx - 1, &map, &factions, &my_faction.name, &mut reactions);
                }
                if pos.x < w - 1 {
                    evaluate(idx + 1, &map, &factions, &my_faction.name, &mut reactions);
                }
                if pos.y > 0 {
                    evaluate(
                        idx - w as usize,
                        &map,
                        &factions,
                        &my_faction.name,
                        &mut reactions,
                    );
                }
                if pos.y < h - 1 {
                    evaluate(
                        idx + w as usize,
                        &map,
                        &factions,
                        &my_faction.name,
                        &mut reactions,
                    );
                }
                if pos.y > 0 && pos.x > 0 {
                    evaluate(
                        (idx - w as usize) - 1,
                        &map,
                        &factions,
                        &my_faction.name,
                        &mut reactions,
                    );
                }
                if pos.y > 0 && pos.x < w - 1 {
                    evaluate(
                        (idx - w as usize) + 1,
                        &map,
                        &factions,
                        &my_faction.name,
                        &mut reactions,
                    );
                }
                if pos.y < h - 1 && pos.x > 0 {
                    evaluate(
                        (idx + w as usize) - 1,
                        &map,
                        &factions,
                        &my_faction.name,
                        &mut reactions,
                    );
                }
                if pos.y < h - 1 && pos.x < w - 1 {
                    evaluate(
                        (idx + w as usize) + 1,
                        &map,
                        &factions,
                        &my_faction.name,
                        &mut reactions,
                    );
                }
            }

            let mut done = false;
            for reaction in &reactions {
                if reaction.1 == Reaction::Attack {
                    want_melee
                        .insert(entity, WantsToMelee { target: reaction.0 })
                        .expect("Error inserting melee");
                    done = true;
                }
            }

            if done {
                turn_done.push(entity);
            }
        }
        for done in &turn_done {
            turns.remove(*done);
        }
    }
}

fn evaluate(
    idx: usize,
    _map: &Map,
    factions: &ReadStorage<Faction>,
    my_faction: &str,
    reactions: &mut Vec<(Entity, Reaction)>,
) {
    spatial::for_each_tile_content(idx, |other_entity| {
        if let Some(faction) = factions.get(other_entity) {
            reactions.push((
                other_entity,
                faction_reaction(my_faction, &faction.name, &RAWS.lock().unwrap()),
            ));
        }
    });
}
