use specs::{Entities, Entity, Join, ReadExpect, ReadStorage, System, WriteStorage};

use crate::components::{Faction, MyTurn, Position, WantsToMelee};
use crate::map::Map;
use crate::raws::rawmaster::{faction_reaction, RAWS};
use crate::raws::Reaction;
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
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut turns, factions, positions, map, mut want_melee, entities, player) = data;

        let mut turn_done = vec![];
        for (entity, _turn, my_faction, pos) in (&entities, &turns, &factions, &positions).join() {
            if entity == *player {
                continue;
            }

            let mut reactions = vec![];
            let idx = map.xy_idx(pos.x, pos.y);
            let w = map.width;
            let h = map.height;

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
