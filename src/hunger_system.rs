use rltk::{ORANGE, RED};
use specs::{Entities, Entity, Join, ReadExpect, ReadStorage, System, WriteStorage};

use crate::components::HungerState::{Hungry, Normal, Starving, WellFed};
use crate::components::{HungerClock, MyTurn};
use crate::effects::{add_effect, EffectType, Targets};
use crate::gamelog;

pub struct HungerSystem {}

impl<'a> System<'a> for HungerSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, HungerClock>,
        ReadExpect<'a, Entity>,
        ReadStorage<'a, MyTurn>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (entities, mut hunger_clock, player_entity, turns) = data;

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
                        gamelog::Logger::new()
                            .color(ORANGE)
                            .append("You are no longer well fed")
                            .log();
                    }
                }
                Normal => {
                    clock.state = Hungry;
                    clock.duration = 200;
                    if entity == *player_entity {
                        gamelog::Logger::new()
                            .color(ORANGE)
                            .append("You are hungry")
                            .log();
                    }
                }
                Hungry => {
                    clock.state = Starving;
                    clock.duration = 200;
                    if entity == *player_entity {
                        gamelog::Logger::new()
                            .color(RED)
                            .append("You are starving!")
                            .log();
                    }
                }
                Starving => {
                    if entity == *player_entity {
                        gamelog::Logger::new()
                            .color(RED)
                            .append(
                                "Your hunger pangs are getting painful! You suffer 1hp of damage.",
                            )
                            .log();
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
