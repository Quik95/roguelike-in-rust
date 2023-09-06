use specs::{Entities, Join, ReadExpect, System, WriteStorage};

use crate::components::{Confusion, MyTurn};
use crate::player::RunState;

pub struct TurnStatusSystem {}

impl<'a> System<'a> for TurnStatusSystem {
    type SystemData = (
        WriteStorage<'a, MyTurn>,
        WriteStorage<'a, Confusion>,
        Entities<'a>,
        ReadExpect<'a, RunState>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut turns, mut confusion, entities, runstate) = data;

        if *runstate != RunState::Ticking {
            return;
        }

        let mut not_my_turn = vec![];
        let mut not_confused = vec![];
        for (entity, _turn, confused) in (&entities, &mut turns, &mut confusion).join() {
            confused.turns -= 1;
            if confused.turns < 1 {
                not_confused.push(entity);
            } else {
                not_my_turn.push(entity);
            }
        }

        for e in not_my_turn {
            turns.remove(e);
        }

        for e in not_confused {
            confusion.remove(e);
        }
    }
}
