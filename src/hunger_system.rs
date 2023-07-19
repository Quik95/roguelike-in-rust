use specs::{Entities, Entity, Join, ReadExpect, ReadStorage, System, WriteExpect, WriteStorage};
use crate::components::{HungerClock, SufferDamage};
use crate::gamelog::GameLog;
use crate::player::RunState;
use crate::player::RunState::*;
use crate::components::HungerState::*;

pub struct HungerSystem{}

impl<'a> System<'a> for HungerSystem {
   type SystemData = (
    Entities<'a>,
       WriteStorage<'a, HungerClock>,
       ReadExpect<'a, Entity>,
       ReadExpect<'a, RunState>,
       WriteStorage<'a, SufferDamage>,
       WriteExpect<'a, GameLog>
   );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut hunger_clock,
            player_entity,
            runstate,
            mut inflict_damage,
            mut log
        ) = data;

        for (entity, mut clock) in (&entities, &mut hunger_clock).join() {
            let mut proceed = false;

            match *runstate {
                PlayerTurn => {
                   if entity == *player_entity {
                       proceed = true;
                   }
                }
                MonsterTurn => {
                    if entity != *player_entity {
                        proceed = true;
                    }
                }
                _  => proceed = false
            }

            if !proceed { continue; }

            clock.duration -= 1;
            if clock.duration >= 1 {continue}

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
                        log.entries.push("Your starvation has ended.".to_string());
                    }
                    SufferDamage::new_damage(&mut inflict_damage, entity, 1);
                }
            }
        }
    }
}