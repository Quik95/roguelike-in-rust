use specs::{Entities, Entity, Join, ReadExpect, ReadStorage, System, WriteStorage};

use crate::components::{
    Chasing, Faction, MyTurn, Position, Viewshed, WantsToApproach, WantsToFlee,
};
use crate::map::Map;
use crate::raws::rawmaster::{faction_reaction, RAWS};
use crate::raws::Reaction;

pub struct VisibleAI {}

impl<'a> System<'a> for VisibleAI {
    type SystemData = (
        ReadStorage<'a, MyTurn>,
        ReadStorage<'a, Faction>,
        ReadStorage<'a, Position>,
        ReadExpect<'a, Map>,
        WriteStorage<'a, WantsToApproach>,
        WriteStorage<'a, WantsToFlee>,
        Entities<'a>,
        ReadExpect<'a, Entity>,
        ReadStorage<'a, Viewshed>,
        WriteStorage<'a, Chasing>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            turns,
            factions,
            positions,
            map,
            mut want_approach,
            mut want_flee,
            entities,
            player,
            viewsheds,
            mut chasing,
        ) = data;

        for (entity, _turn, my_faction, pos, viewshed) in
            (&entities, &turns, &factions, &positions, &viewsheds).join()
        {
            if entity == *player {
                continue;
            }

            let my_idx = map.xy_idx(pos.x, pos.y);
            let mut reactions = vec![];
            let mut flee = vec![];
            for visible_tile in viewshed.visible_tiles.iter() {
                let idx = map.xy_idx(visible_tile.x, visible_tile.y);
                if my_idx != idx {
                    evaluate(idx, &map, &factions, &my_faction.name, &mut reactions);
                }
            }

            let mut done = false;
            for reaction in reactions.iter() {
                match reaction.1 {
                    Reaction::Attack => {
                        want_approach
                            .insert(
                                entity,
                                WantsToApproach {
                                    idx: reaction.0 as i32,
                                },
                            )
                            .expect("Unable to insert");
                        chasing
                            .insert(entity, Chasing { target: reaction.2 })
                            .expect("Unable to insert");
                        done = true;
                    }
                    Reaction::Flee => {
                        flee.push(reaction.0);
                    }
                    _ => {}
                }
            }

            if !done && !flee.is_empty() {
                want_flee
                    .insert(entity, WantsToFlee { indices: flee })
                    .expect("Unable to insert");
            }
        }
    }
}

fn evaluate(
    idx: usize,
    map: &Map,
    factions: &ReadStorage<Faction>,
    my_faction: &str,
    reactions: &mut Vec<(usize, Reaction, Entity)>,
) {
    for other_entity in map.tile_content[idx].iter() {
        if let Some(faction) = factions.get(*other_entity) {
            reactions.push((
                idx,
                faction_reaction(my_faction, &faction.name, &RAWS.lock().unwrap()),
                *other_entity,
            ));
        }
    }
}
