use crate::map_builders::common::draw_corridor;
use crate::map_builders::{BuilderMap, MetaMapBuilder};
use crate::rng::roll_dice;

pub struct BspCorridors {}

impl MetaMapBuilder for BspCorridors {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.corridors(build_data);
    }
}

impl BspCorridors {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn corridors(&mut self, build_data: &mut BuilderMap) {
        let rooms = build_data.rooms.as_ref().map_or_else(
            || panic!("BSP Corridors require a builder with room structures"),
            std::clone::Clone::clone,
        );

        let mut corridors = Vec::new();
        for i in 0..rooms.len() - 1 {
            let room = rooms[i];
            let next_room = rooms[i + 1];
            let start_x = room.x1 + (roll_dice(1, i32::abs(room.x1 - room.x2)) - 1);
            let start_y = room.y1 + (roll_dice(1, i32::abs(room.y1 - room.y2)) - 1);
            let end_x = next_room.x1 + (roll_dice(1, i32::abs(next_room.x1 - next_room.x2)) - 1);
            let end_y = next_room.y1 + (roll_dice(1, i32::abs(next_room.y1 - next_room.y2)) - 1);
            let corridor = draw_corridor(&mut build_data.map, start_x, start_y, end_x, end_y);
            corridors.push(corridor);
            build_data.take_snapshot();
        }

        build_data.corridors = Some(corridors);
    }
}
