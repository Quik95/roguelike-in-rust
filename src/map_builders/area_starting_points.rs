use rltk::{Point, RandomNumberGenerator};
use rltk::DistanceAlg::PythagorasSquared;

use crate::components::Position;
use crate::map::TileType;
use crate::map_builders::{BuilderMap, MetaMapBuilder};

pub enum XStart { Left, Center, Right }

pub enum YStart { Top, Center, Bottom }

pub struct AreaStartingPosition {
    x: XStart,
    y: YStart,
}

impl MetaMapBuilder for AreaStartingPosition {
    fn build_map(&mut self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        self.build(rng, build_data);
    }
}

impl AreaStartingPosition {
    pub fn new(x: XStart, y: YStart) -> Box<Self> {
        Box::new(Self { x, y })
    }

    fn build(&mut self, _: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        let seed_x = match self.x {
            XStart::Left => 1,
            XStart::Center => build_data.map.width / 2,
            XStart::Right => build_data.map.width - 2
        };

        let seed_y = match self.y {
            YStart::Top => 1,
            YStart::Center => build_data.map.height / 2,
            YStart::Bottom => build_data.map.height - 2
        };

        let mut available_floors = Vec::new();
        for (idx, tiletype) in build_data.map.tiles.iter().enumerate() {
            if *tiletype == TileType::Floor {
                available_floors.push(
                    (
                        idx,
                        PythagorasSquared.distance2d(
                            Point::new(idx as i32 % build_data.map.width, idx as i32 / build_data.map.width),
                            Point::new(seed_x, seed_y),
                        )
                    )
                );
            }
        }
        if available_floors.is_empty() {
            panic!("No valid floors to start on");
        }

        available_floors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let start_x = available_floors[0].0 as i32 % build_data.map.width;
        let start_y = available_floors[0].0 as i32 / build_data.map.width;

        build_data.starting_position = Some(Position { x: start_x, y: start_y });
    }
}