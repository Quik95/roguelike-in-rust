use crate::components::Position;
use crate::map::tiletype::TileType;
use crate::map_builders::common::{paint, Symmetry};
use crate::map_builders::{BuilderMap, InitialMapBuilder, MetaMapBuilder};
use crate::rng::roll_dice;

pub struct DrunkardsWalkBuilder {
    settings: DrunkardSettings,
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum DrunkSpawnMode {
    StartingPoint,
    Random,
}

pub struct DrunkardSettings {
    pub spawn_mode: DrunkSpawnMode,
    pub drunken_lifetime: i32,
    pub floor_percent: f32,
    pub brush_size: i32,
    pub symmetry: Symmetry,
}

impl InitialMapBuilder for DrunkardsWalkBuilder {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl MetaMapBuilder for DrunkardsWalkBuilder {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl DrunkardsWalkBuilder {
    pub fn new(settings: DrunkardSettings) -> Box<Self> {
        Box::new(Self { settings })
    }

    pub fn open_area() -> Box<Self> {
        let settings = DrunkardSettings {
            spawn_mode: DrunkSpawnMode::StartingPoint,
            drunken_lifetime: 400,
            floor_percent: 0.5,
            brush_size: 1,
            symmetry: Symmetry::None,
        };

        Self::new(settings)
    }

    pub fn open_halls() -> Box<Self> {
        let settings = DrunkardSettings {
            spawn_mode: DrunkSpawnMode::Random,
            drunken_lifetime: 400,
            floor_percent: 0.5,

            brush_size: 1,
            symmetry: Symmetry::None,
        };

        Self::new(settings)
    }

    pub fn winding_passages() -> Box<Self> {
        let settings = DrunkardSettings {
            spawn_mode: DrunkSpawnMode::Random,
            drunken_lifetime: 100,
            floor_percent: 0.4,
            brush_size: 1,
            symmetry: Symmetry::None,
        };

        Self::new(settings)
    }

    pub fn fat_passages() -> Box<Self> {
        let settings = DrunkardSettings {
            spawn_mode: DrunkSpawnMode::Random,
            drunken_lifetime: 100,
            floor_percent: 0.4,
            brush_size: 2,
            symmetry: Symmetry::None,
        };

        Self::new(settings)
    }

    pub fn fearful_symmetry() -> Box<Self> {
        let settings = DrunkardSettings {
            spawn_mode: DrunkSpawnMode::Random,
            drunken_lifetime: 100,
            floor_percent: 0.4,
            brush_size: 1,
            symmetry: Symmetry::Both,
        };

        Self::new(settings)
    }

    fn build(&mut self, build_data: &mut BuilderMap) {
        let starting_position = Position {
            x: build_data.map.width / 2,
            y: build_data.map.height / 2,
        };
        let start_idx = build_data
            .map
            .xy_idx(starting_position.x, starting_position.y);
        build_data.map.tiles[start_idx] = TileType::Floor;

        let total_tiles = build_data.map.width * build_data.map.height;
        let desired_floor_tiles = (self.settings.floor_percent * total_tiles as f32) as usize;
        let mut floor_tile_count = build_data
            .map
            .tiles
            .iter()
            .filter(|a| **a == TileType::Floor)
            .count();
        let mut digger_count = 0;

        while floor_tile_count < desired_floor_tiles {
            let mut did_something = false;
            let mut drunk_x;
            let mut drunk_y;

            match self.settings.spawn_mode {
                DrunkSpawnMode::StartingPoint => {
                    drunk_x = starting_position.x;
                    drunk_y = starting_position.y;
                }
                DrunkSpawnMode::Random => {
                    if digger_count == 0 {
                        drunk_x = starting_position.x;
                        drunk_y = starting_position.y;
                    } else {
                        drunk_x = roll_dice(1, build_data.map.width - 3) + 1;
                        drunk_y = roll_dice(1, build_data.map.height - 3) + 1;
                    }
                }
            }

            let mut drunk_life = self.settings.drunken_lifetime;

            while drunk_life > 0 {
                let drunk_idx = build_data.map.xy_idx(drunk_x, drunk_y);
                if build_data.map.tiles[drunk_idx] == TileType::Wall {
                    did_something = true;
                }
                paint(
                    &mut build_data.map,
                    self.settings.symmetry,
                    self.settings.brush_size,
                    drunk_x,
                    drunk_y,
                );
                build_data.map.tiles[drunk_idx] = TileType::DownStairs;

                let stagger_direction = roll_dice(1, 4);
                match stagger_direction {
                    1 => {
                        if drunk_x > 2 {
                            drunk_x -= 1;
                        }
                    }
                    2 => {
                        if drunk_x < build_data.map.width - 2 {
                            drunk_x += 1;
                        }
                    }
                    3 => {
                        if drunk_y > 2 {
                            drunk_y -= 1;
                        }
                    }
                    4 => {
                        if drunk_y < build_data.map.height - 2 {
                            drunk_y += 1;
                        }
                    }
                    _ => unreachable!(),
                }

                drunk_life -= 1;
            }
            if did_something {
                build_data.take_snapshot();
            }

            digger_count += 1;
            for t in &mut build_data.map.tiles {
                if *t == TileType::DownStairs {
                    *t = TileType::Floor;
                }
            }
            floor_tile_count = build_data
                .map
                .tiles
                .iter()
                .filter(|a| **a == TileType::Floor)
                .count();
        }
    }
}
