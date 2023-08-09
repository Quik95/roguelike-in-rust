use rltk::RandomNumberGenerator;

use crate::map_builders::{BuilderMap, MetaMapBuilder};
use crate::map_builders::common::{apply_horizontal_tunnel, apply_vertical_tunnel};

pub struct DoglegCorridors {}

impl MetaMapBuilder for DoglegCorridors {
    fn build_map(&mut self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        self.corridors(rng, build_data);
    }
}

impl DoglegCorridors {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn corridors(&mut self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        let rooms = build_data.rooms.as_ref().map_or_else(
            || panic!("Dogleg Corridors require a builder with room structure."),
            |room_builder| room_builder.clone(),
        );

        for (i, room) in rooms.iter().enumerate() {
            if i > 0 {
                let (new_x, new_y)  = room.center();
                let (prev_x, prev_y) = rooms[i as usize - 1].center();
                if rng.range(0, 2) == 1{
                    apply_horizontal_tunnel(&mut build_data.map, prev_x, new_x, prev_y);
                    apply_vertical_tunnel(&mut build_data.map, prev_y, new_y, new_x);
                } else {
                    apply_vertical_tunnel(&mut build_data.map, prev_y, new_y, prev_x);
                    apply_horizontal_tunnel(&mut build_data.map, prev_x, new_x, new_y);
                }
                build_data.take_snapshot();
            }
        }
    }
}