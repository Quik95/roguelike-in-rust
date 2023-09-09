use specs::{Entities, Entity, Join, ReadExpect, ReadStorage, System, WriteExpect, WriteStorage};

use crate::components::HungerState::*;
use crate::components::{HungerClock, MyTurn};
use crate::effects::{add_effect, EffectType, Targets};
use crate::gamelog::GameLog;

pub struct HungerSystem {}

impl<'a> System<'a> for HungerSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, HungerClock>,
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        ReadStorage<'a, MyTurn>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (entities, mut hunger_clock, player_entity, mut log, turns) = data;

        for (entity, clock, _myturn) in (&entities, &mut hunger_clock, &turns).join() {
            clock.duration -= 1;
            if clock.duration >= 1 {
                continue;
            }

            match clock.state {
                WellFed => {
                    clock.state = Normal;
                    clock.duration = 200;
                    if entity == *player_entity {
                        log.entries.push("You are no longer well fed.".to_string());
                    }
                }
                Normal => {
                    clock.state = Hungry;
                    clock.duration = 200;
                    if entity == *player_entity {
                        log.entries.push("You are hungry.".to_string());
                    }
                }
                Hungry => {
                    clock.state = Starving;
                    clock.duration = 200;
                    if entity == *player_entity {
                        log.entries.push("You are starving!".to_string());
                    }
                }
                Starving => {
                    if entity == *player_entity {
                        log.entries.push(
                            "Your hunger pangs are getting painful! Your suffer 1 hp of damage."
                                .to_string(),
                        );
                    }
                    add_effect(
                        None,
                        EffectType::Damage { amount: 1 },
                        Targets::Single { target: entity },
                    );
                }
            }
        }
    }
}
