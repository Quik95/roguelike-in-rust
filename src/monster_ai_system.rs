use rltk::{console, Point};
use specs::{Join, ReadExpect, ReadStorage, System};

use crate::components::{Monster, Name, Position, Viewshed};

pub struct MonsterAI {}
impl<'a> System<'a> for MonsterAI {
    type SystemData = (
        ReadStorage<'a, Viewshed>,
        ReadExpect<'a, Point>,
        ReadStorage<'a, Monster>,
        ReadStorage<'a, Name>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (viewshed, pos, monster, name) = data;

        for (viewshed, _monster, name) in (&viewshed, &monster, &name).join() {
            if viewshed.visible_tiles.contains(&*pos) {
                console::log(&format!("{} shouts insults", name.name));
            }
        }
    }
}
