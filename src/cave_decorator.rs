use crate::map::tiletype::TileType;
use crate::map_builders::{BuilderMap, MetaMapBuilder};
use crate::rng::roll_dice;

pub struct CaveDecorator {}

impl MetaMapBuilder for CaveDecorator {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl CaveDecorator {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    pub fn build(&mut self, build_data: &mut BuilderMap) {
        let old_map = build_data.map.clone();
        for (idx, tt) in build_data.map.tiles.iter_mut().enumerate() {
            if *tt == TileType::Floor && roll_dice(1, 6) == 1 {
                *tt = TileType::Gravel;
            } else if *tt == TileType::Floor && roll_dice(1, 10) == 1 {
                *tt = TileType::ShallowWater;
            } else if *tt == TileType::Wall {
                let mut neighbors = 0;
                let x = idx as i32 % old_map.width;
                let y = idx as i32 / old_map.width;
                if x > 0 && old_map.tiles[idx - 1] == TileType::Wall {
                    neighbors += 1;
                }
                if x < old_map.width - 2 && old_map.tiles[idx + 1] == TileType::Wall {
                    neighbors += 1;
                }
                if y > 0 && old_map.tiles[idx - old_map.width as usize] == TileType::Wall {
                    neighbors += 1;
                }
                if y < old_map.height - 2
                    && old_map.tiles[idx + old_map.width as usize] == TileType::Wall
                {
                    neighbors += 1;
                }
                if neighbors == 2 {
                    *tt = TileType::DeepWater;
                } else if neighbors == 1 {
                    let roll = roll_dice(1, 4);
                    match roll {
                        1 => *tt = TileType::Stalactite,
                        2 => *tt = TileType::Stalagmite,
                        3 | 4 => {}
                        _ => unreachable!(),
                    }
                }
            }
        }

        build_data.take_snapshot();
        build_data.map.natural_light = false;
    }
}
