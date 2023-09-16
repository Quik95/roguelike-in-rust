use rltk::{DistanceAlg, Point};
use specs::{Entities, Entity, Join, ReadExpect, ReadStorage, System, WriteExpect, WriteStorage};

use crate::components::{
    Attributes, DamageOverTime, Duration, EquipmentChanged, Initiative, MyTurn, Pools, Position,
    StatusEffect,
};
use crate::effects::{add_effect, EffectType, Targets};
use crate::player::RunState;
use crate::rng::roll_dice;

pub struct InitiativeSystem {}

impl<'a> System<'a> for InitiativeSystem {
    type SystemData = (
        WriteStorage<'a, Initiative>,
        ReadStorage<'a, Position>,
        WriteStorage<'a, MyTurn>,
        Entities<'a>,
        ReadStorage<'a, Attributes>,
        WriteExpect<'a, RunState>,
        ReadExpect<'a, Entity>,
        ReadExpect<'a, Point>,
        ReadStorage<'a, Pools>,
        WriteStorage<'a, Duration>,
        WriteStorage<'a, EquipmentChanged>,
        ReadStorage<'a, StatusEffect>,
        ReadStorage<'a, DamageOverTime>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut initiatives,
            positions,
            mut turns,
            entities,
            attributes,
            mut runstate,
            player,
            player_pos,
            pools,
            mut durations,
            mut dirty,
            statuses,
            dots,
        ) = data;

        if *runstate != RunState::Ticking {
            return;
        }

        turns.clear();

        for (entity, initiative, pos) in (&entities, &mut initiatives, &positions).join() {
            initiative.current -= 1;
            if initiative.current < 1 {
                let mut myturn = true;

                initiative.current = 6 + roll_dice(1, 6);

                if let Some(attr) = attributes.get(entity) {
                    initiative.current -= attr.quickness.bonus;
                }

                if let Some(pools) = pools.get(entity) {
                    initiative.current += f32::floor(pools.total_initiative_penalty) as i32;
                }

                if entity == *player {
                    *runstate = RunState::AwaitingInput;
                } else {
                    let distance =
                        DistanceAlg::Pythagoras.distance2d(*player_pos, Point::new(pos.x, pos.y));
                    if distance > 20.0 {
                        myturn = false;
                    }
                }

                if myturn {
                    turns
                        .insert(entity, MyTurn {})
                        .expect("Unable to insert turn");
                }
            }
        }

        if *runstate == RunState::AwaitingInput {
            for (effect_entity, duration, status) in (&entities, &mut durations, &statuses).join() {
                if !entities.is_alive(status.target) {
                    continue;
                }

                duration.turns -= 1;

                if let Some(dot) = dots.get(effect_entity) {
                    add_effect(
                        None,
                        EffectType::Damage { amount: dot.damage },
                        Targets::Single {
                            target: status.target,
                        },
                    );
                }

                if duration.turns < 1 {
                    dirty
                        .insert(status.target, EquipmentChanged {})
                        .expect("Unable to insert");
                    entities.delete(effect_entity).expect("Unable to delete");
                }
            }
        }
    }
}
