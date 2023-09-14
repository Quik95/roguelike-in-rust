use rltk::{console, DistanceAlg, Point, RandomNumberGenerator};
use specs::{Entities, Entity, Join, ReadExpect, ReadStorage, System, WriteExpect, WriteStorage};

use crate::components::{
    Chasing, Equipped, Faction, MyTurn, Name, Position, SpecialAbilities, SpellTemplate, Viewshed,
    WantsToApproach, WantsToCastSpell, WantsToFlee, WantsToShoot, Weapon,
};
use crate::map::Map;
use crate::raws::rawmaster::{faction_reaction, find_spell_entity_by_name, RAWS};
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
        ReadStorage<'a, SpecialAbilities>,
        WriteExpect<'a, RandomNumberGenerator>,
        WriteStorage<'a, WantsToCastSpell>,
        ReadStorage<'a, Name>,
        ReadStorage<'a, SpellTemplate>,
        ReadStorage<'a, Equipped>,
        ReadStorage<'a, Weapon>,
        WriteStorage<'a, WantsToShoot>,
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
            abilities,
            mut rng,
            mut casting,
            names,
            spells,
            equipped,
            weapons,
            mut wants_shoot,
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
            for visible_tile in &viewshed.visible_tiles {
                let idx = map.xy_idx(visible_tile.x, visible_tile.y);
                if my_idx != idx {
                    evaluate(idx, &map, &factions, &my_faction.name, &mut reactions);
                }
            }

            let mut done = false;
            for reaction in &reactions {
                match reaction.1 {
                    Reaction::Attack => {
                        let range = DistanceAlg::Pythagoras.distance2d(
                            Point::new(pos.x, pos.y),
                            Point::new(
                                reaction.0 as i32 % map.width,
                                reaction.0 as i32 / map.width,
                            ),
                        );
                        if let Some(abilities) = abilities.get(entity) {
                            for ability in abilities.abilities.iter() {
                                if range >= ability.min_range
                                    && range <= ability.range
                                    && rng.roll_dice(1, 100) >= (ability.chance * 100.0) as i32
                                {
                                    casting
                                        .insert(
                                            entity,
                                            WantsToCastSpell {
                                                spell: find_spell_entity_by_name(
                                                    &ability.spell,
                                                    &names,
                                                    &spells,
                                                    &entities,
                                                )
                                                .unwrap(),
                                                target: Some(Point::new(
                                                    reaction.0 as i32 % map.width,
                                                    reaction.0 as i32 / map.width,
                                                )),
                                            },
                                        )
                                        .expect("Unable to insert");
                                    done = true;
                                }
                            }
                        }
                        if done {
                            continue;
                        }

                        for (weapon, equip) in (&weapons, &equipped).join() {
                            if let Some(wrange) = weapon.range {
                                if equip.owner == entity {
                                    console::log(format!(
                                        "Owner found. Ranges: {}/{}",
                                        wrange, range
                                    ));
                                    if wrange >= range as i32 {
                                        console::log("Inserting shoot");
                                        wants_shoot
                                            .insert(entity, WantsToShoot { target: reaction.2 })
                                            .expect("Insert fail");
                                        done = true;
                                    }
                                }
                            }
                        }

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
    _map: &Map,
    factions: &ReadStorage<Faction>,
    my_faction: &str,
    reactions: &mut Vec<(usize, Reaction, Entity)>,
) {
    crate::spatial::for_each_tile_content(idx, |other_entity| {
        if let Some(faction) = factions.get(other_entity) {
            reactions.push((
                idx,
                faction_reaction(my_faction, &faction.name, &RAWS.lock().unwrap()),
                other_entity,
            ));
        }
    });
}
