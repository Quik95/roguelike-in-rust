use rltk::{Point, RandomNumberGenerator};
use specs::{Join, ReadExpect, ReadStorage, System, WriteExpect, WriteStorage};

use crate::components::{MyTurn, Name, Quips, Viewshed};
use crate::gamelog::GameLog;

pub struct QuipSystem {}

impl<'a> System<'a> for QuipSystem {
    type SystemData = (
        WriteExpect<'a, GameLog>,
        WriteStorage<'a, Quips>,
        ReadStorage<'a, Name>,
        ReadStorage<'a, MyTurn>,
        ReadExpect<'a, Point>,
        ReadStorage<'a, Viewshed>,
        WriteExpect<'a, RandomNumberGenerator>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut gamelog, mut quips, names, turns, player_pos, viewsheds, mut rng) = data;

        for (quip, name, viewshed, _turn) in (&mut quips, &names, &viewsheds, &turns).join() {
            if !quip.available.is_empty()
                && viewshed.visible_tiles.contains(&player_pos)
                && rng.roll_dice(1, 6) == 1
            {
                let quip_index = if quip.available.len() == 1 {
                    0
                } else {
                    (rng.roll_dice(1, quip.available.len() as i32) - 1) as usize
                };

                gamelog.entries.push(format!(
                    "{} says \"{}\"",
                    name.name, quip.available[quip_index]
                ));
                quip.available.remove(quip_index);
            }
        }
    }
}
