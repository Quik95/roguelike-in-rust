use rltk::Point;
use specs::{Join, ReadExpect, ReadStorage, System, WriteStorage};

use crate::components::{MyTurn, Name, Quips, Viewshed};
use crate::gamelog;
use crate::rng::roll_dice;

pub struct QuipSystem {}

impl<'a> System<'a> for QuipSystem {
    type SystemData = (
        WriteStorage<'a, Quips>,
        ReadStorage<'a, Name>,
        ReadStorage<'a, MyTurn>,
        ReadExpect<'a, Point>,
        ReadStorage<'a, Viewshed>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut quips, names, turns, player_pos, viewsheds) = data;

        for (quip, name, viewshed, _turn) in (&mut quips, &names, &viewsheds, &turns).join() {
            if !quip.available.is_empty()
                && viewshed.visible_tiles.contains(&player_pos)
                && roll_dice(1, 6) == 1
            {
                let quip_index = if quip.available.len() == 1 {
                    0
                } else {
                    (roll_dice(1, quip.available.len() as i32) - 1) as usize
                };

                gamelog::Logger::new()
                    .npc_name(&name.name)
                    .append("says")
                    .npc_name(&quip.available[quip_index])
                    .log();
                quip.available.remove(quip_index);
            }
        }
    }
}
