use rltk::{DistanceAlg, Point, RandomNumberGenerator};
use specs::{Entities, Entity, Join, ReadExpect, ReadStorage, System, WriteExpect, WriteStorage};

use crate::components::{Attributes, Initiative, MyTurn, Pools, Position};
use crate::player::RunState;

pub struct InitiativeSystem {}

impl<'a> System<'a> for InitiativeSystem {
    type SystemData = (
        WriteStorage<'a, Initiative>,
        ReadStorage<'a, Position>,
        WriteStorage<'a, MyTurn>,
        Entities<'a>,
        WriteExpect<'a, RandomNumberGenerator>,
        ReadStorage<'a, Attributes>,
        WriteExpect<'a, RunState>,
        ReadExpect<'a, Entity>,
        ReadExpect<'a, Point>,
        ReadStorage<'a, Pools>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut initiatives,
            positions,
            mut turns,
            entities,
            mut rng,
            attributes,
            mut runstate,
            player,
            player_pos,
            pools,
        ) = data;

        if *runstate != RunState::Ticking {
            return;
        }

        turns.clear();

        for (entity, initiative, pos) in (&entities, &mut initiatives, &positions).join() {
            initiative.current -= 1;
            if initiative.current < 1 {
                let mut myturn = true;

                initiative.current = 6 + rng.roll_dice(1, 6);

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
    }
}
