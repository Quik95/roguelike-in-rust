use std::collections::HashSet;

use rltk::{to_cp437, BLACK, CYAN};
use specs::{Entities, Join, ReadExpect, ReadStorage, System, WriteStorage};

use crate::components::{Confusion, MyTurn, StatusEffect};
use crate::effects::{add_effect, EffectType, Targets};
use crate::player::RunState;

pub struct TurnStatusSystem {}

impl<'a> System<'a> for TurnStatusSystem {
    type SystemData = (
        WriteStorage<'a, MyTurn>,
        WriteStorage<'a, Confusion>,
        Entities<'a>,
        ReadExpect<'a, RunState>,
        ReadStorage<'a, StatusEffect>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut turns, confusion, entities, runstate, statuses) = data;

        if *runstate != RunState::Ticking {
            return;
        }

        let mut entity_turns = HashSet::new();
        for (entity, _turn) in (&entities, &turns).join() {
            entity_turns.insert(entity);
        }

        let mut not_my_turn = vec![];
        for (effect_entity, status_effect) in (&entities, &statuses).join() {
            if entity_turns.contains(&status_effect.target) {
                if confusion.get(effect_entity).is_some() {
                    add_effect(
                        None,
                        EffectType::Particle {
                            glyph: to_cp437('?'),
                            fg: CYAN.into(),
                            bg: BLACK.into(),
                            lifespan: 200.0,
                        },
                        Targets::Single {
                            target: status_effect.target,
                        },
                    );
                }
                not_my_turn.push(status_effect.target);
            }
        }

        for e in not_my_turn {
            turns.remove(e);
        }
    }
}
