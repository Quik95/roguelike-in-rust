use rltk::RandomNumberGenerator;

use crate::map::TileType;
use crate::map_builders::{BuilderMap, MetaMapBuilder};

pub struct RoomBasedStairs {}

impl MetaMapBuilder for RoomBasedStairs {
    fn build_map(&mut self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        self.build(rng, build_data);
    }
}

impl RoomBasedStairs {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn build(&mut self, _: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        if let Some(rooms) = &build_data.rooms {
            let stairs_position = rooms[rooms.len() - 1].center();
            let stairs_idx = build_data.map.xy_idx(stairs_position.0, stairs_position.1);
            build_data.map.tiles[stairs_idx] = TileType::DownStairs;
            build_data.take_snapshot();
        } else {
            panic!("Room Based Stairs only works after rooms have been created.");
        }
    }
}