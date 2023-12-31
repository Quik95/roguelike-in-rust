use crate::map::tiletype::TileType;
use crate::map_builders::common::{paint, Symmetry};
use crate::map_builders::{BuilderMap, MetaMapBuilder};
use crate::rng::roll_dice;

pub struct RoomExploder {}

impl RoomExploder {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    pub(crate) fn build(&self, build_data: &mut BuilderMap) {
        let rooms = build_data.rooms.as_ref().map_or_else(
            || {
                panic!("Room Explosion requires a builder with room structures");
            },
            std::clone::Clone::clone,
        );

        for room in &rooms {
            let start = room.center();
            let n_diggers = roll_dice(1, 20) - 5;
            if n_diggers > 0 {
                for _ in 0..n_diggers {
                    let mut drunk_x = start.0;
                    let mut drunk_y = start.1;

                    let mut drunk_life = 20;
                    let mut did_something = false;

                    while drunk_life > 0 {
                        let drunk_idx = build_data.map.xy_idx(drunk_x, drunk_y);
                        if build_data.map.tiles[drunk_idx] == TileType::Wall {
                            did_something = true;
                        }
                        paint(&mut build_data.map, Symmetry::None, 1, drunk_x, drunk_y);
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

                    for t in &mut build_data.map.tiles {
                        if *t == TileType::DownStairs {
                            *t = TileType::Floor;
                        }
                    }
                }
            }
        }
    }
}

impl MetaMapBuilder for RoomExploder {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}
