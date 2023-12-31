use rltk::{DistanceAlg, Point};

use crate::map::tiletype::TileType;
use crate::map_builders::{BuilderMap, MetaMapBuilder};
use crate::rect::Rect;
use crate::rng::roll_dice;

pub struct RoomDrawer {}

impl MetaMapBuilder for RoomDrawer {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl RoomDrawer {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn rectangle(&mut self, build_data: &mut BuilderMap, room: &Rect) {
        for y in room.y1 + 1..=room.y2 {
            for x in room.x1 + 1..=room.x2 {
                let idx = build_data.map.xy_idx(x, y);
                if idx > 0 && idx < ((build_data.map.width * build_data.map.height) - 1) as usize {
                    build_data.map.tiles[idx] = TileType::Floor;
                }
            }
        }
    }

    fn circle(&mut self, build_data: &mut BuilderMap, room: &Rect) {
        let radius = i32::min(room.x2 - room.x1, room.y2 - room.y1) as f32 / 2.0;
        let center = room.center();
        let center_pt = Point::new(center.0, center.1);
        for y in room.y1..=room.y2 {
            for x in room.x1..=room.x2 {
                let idx = build_data.map.xy_idx(x, y);
                let distance = DistanceAlg::Pythagoras.distance2d(center_pt, Point::new(x, y));
                if idx > 0
                    && idx < ((build_data.map.width * build_data.map.height) - 1) as usize
                    && distance <= radius
                {
                    build_data.map.tiles[idx] = TileType::Floor;
                }
            }
        }
    }

    fn build(&mut self, build_data: &mut BuilderMap) {
        let rooms = build_data.rooms.as_ref().map_or_else(
            || panic!("Room rounding requires a builder with room structures."),
            std::clone::Clone::clone,
        );

        for room in &rooms {
            let room_type = roll_dice(1, 4);
            match room_type {
                1 => self.circle(build_data, room),
                _ => self.rectangle(build_data, room),
            }
            build_data.take_snapshot();
        }
    }
}
