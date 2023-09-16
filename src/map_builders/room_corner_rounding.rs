use crate::map::tiletype::TileType;
use crate::map_builders::{BuilderMap, MetaMapBuilder};

pub struct RoomCornerRounder {}

impl MetaMapBuilder for RoomCornerRounder {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl RoomCornerRounder {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn fill_if_corner(&mut self, x: i32, y: i32, build_data: &mut BuilderMap) {
        let w = build_data.map.width;
        let h = build_data.map.height;
        let idx = build_data.map.xy_idx(x, y);
        let mut neighbor_walls = 0;
        if x > 0 && build_data.map.tiles[idx - 1] == TileType::Wall {
            neighbor_walls += 1;
        }
        if y > 0 && build_data.map.tiles[idx - w as usize] == TileType::Wall {
            neighbor_walls += 1;
        }
        if x < w - 2 && build_data.map.tiles[idx + 1] == TileType::Wall {
            neighbor_walls += 1;
        }
        if y < h - 2 && build_data.map.tiles[idx + w as usize] == TileType::Wall {
            neighbor_walls += 1;
        }

        if neighbor_walls == 2 {
            build_data.map.tiles[idx] = TileType::Wall;
        }
    }

    fn build(&mut self, build_data: &mut BuilderMap) {
        let rooms = build_data.rooms.as_ref().map_or_else(
            || panic!("Room rounding requires a builder with room structures"),
            std::clone::Clone::clone,
        );

        for room in &rooms {
            self.fill_if_corner(room.x1 + 1, room.y1 + 1, build_data);
            self.fill_if_corner(room.x2, room.y1 + 1, build_data);
            self.fill_if_corner(room.x1 + 1, room.y2, build_data);
            self.fill_if_corner(room.x2, room.y2, build_data);

            build_data.take_snapshot();
        }
    }
}
