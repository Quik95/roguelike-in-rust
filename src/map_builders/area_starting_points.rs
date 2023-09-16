use rltk::DistanceAlg::PythagorasSquared;
use rltk::{DistanceAlg, Point};

use crate::components::Position;
use crate::map::tiletype::TileType;
use crate::map_builders::{BuilderMap, MetaMapBuilder};

#[allow(dead_code)]
pub enum XStart {
    Left,
    Center,
    Right,
}

#[allow(dead_code)]
pub enum XEnd {
    Left,
    Center,
    Right,
}

#[allow(dead_code)]
pub enum YStart {
    Top,
    Center,
    Bottom,
}

#[allow(dead_code)]
pub enum YEnd {
    Top,
    Center,
    Bottom,
}

pub struct AreaStartingPosition {
    x: XStart,
    y: YStart,
}

pub struct AreaEndingPosition {
    x: XEnd,
    y: YEnd,
}

impl MetaMapBuilder for AreaStartingPosition {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl MetaMapBuilder for AreaEndingPosition {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl AreaEndingPosition {
    pub fn new(x: XEnd, y: YEnd) -> Box<Self> {
        Box::new(Self { x, y })
    }

    fn build(&mut self, build_data: &mut BuilderMap) {
        let seed_x = match self.x {
            XEnd::Left => 1,
            XEnd::Center => build_data.map.width / 2,
            XEnd::Right => build_data.map.width - 2,
        };

        let seed_y = match self.y {
            YEnd::Top => 1,
            YEnd::Center => build_data.map.height / 2,
            YEnd::Bottom => build_data.map.height - 2,
        };

        let mut available_floors = vec![];
        for (idx, tiletype) in build_data.map.tiles.iter().enumerate() {
            if tiletype.is_walkable() {
                available_floors.push((
                    idx,
                    DistanceAlg::PythagorasSquared.distance2d(
                        Point::new(
                            idx as i32 % build_data.map.width,
                            idx as i32 / build_data.map.width,
                        ),
                        Point::new(seed_x, seed_y),
                    ),
                ));
            }
        }
        assert!(!available_floors.is_empty(), "No valid floors to start on");

        available_floors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        build_data.map.tiles[available_floors[0].0] = TileType::DownStairs;
    }
}

impl AreaStartingPosition {
    pub fn new(x: XStart, y: YStart) -> Box<Self> {
        Box::new(Self { x, y })
    }

    fn build(&mut self, build_data: &mut BuilderMap) {
        let seed_x = match self.x {
            XStart::Left => 1,
            XStart::Center => build_data.map.width / 2,
            XStart::Right => build_data.map.width - 2,
        };

        let seed_y = match self.y {
            YStart::Top => 1,
            YStart::Center => build_data.map.height / 2,
            YStart::Bottom => build_data.map.height - 2,
        };

        let mut available_floors = Vec::new();
        for (idx, tiletype) in build_data.map.tiles.iter().enumerate() {
            if tiletype.is_walkable() {
                available_floors.push((
                    idx,
                    PythagorasSquared.distance2d(
                        Point::new(
                            idx as i32 % build_data.map.width,
                            idx as i32 / build_data.map.width,
                        ),
                        Point::new(seed_x, seed_y),
                    ),
                ));
            }
        }
        assert!(!available_floors.is_empty(), "No valid floors to start on");

        available_floors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let start_x = available_floors[0].0 as i32 % build_data.map.width;
        let start_y = available_floors[0].0 as i32 / build_data.map.width;

        build_data.starting_position = Some(Position {
            x: start_x,
            y: start_y,
        });
    }
}
