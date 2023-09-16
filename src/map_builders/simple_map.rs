use crate::rng::{range, roll_dice};

use crate::map_builders::{BuilderMap, InitialMapBuilder};
use crate::rect::Rect;

const MAX_ROOMS: i32 = 30;
const MIN_SIZE: i32 = 6;
const MAX_SIZE: i32 = 10;

pub struct SimpleMapBuilder {}

impl InitialMapBuilder for SimpleMapBuilder {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build_rooms(build_data);
    }
}

impl SimpleMapBuilder {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn build_rooms(&mut self, build_data: &mut BuilderMap) {
        let mut rooms = Vec::new();

        for _i in 0..MAX_ROOMS {
            let w = range(MIN_SIZE, MAX_SIZE);
            let h = range(MIN_SIZE, MAX_SIZE);
            let x = roll_dice(1, build_data.map.width - w - 1) - 1;
            let y = roll_dice(1, build_data.map.height - h - 1) - 1;
            let new_room = Rect::new(x, y, w, h);
            let mut ok = true;
            for other_room in &rooms {
                if new_room.intersects(other_room) {
                    ok = false;
                }
            }
            if ok {
                rooms.push(new_room);
            }
        }
        build_data.rooms = Some(rooms);
    }
}
