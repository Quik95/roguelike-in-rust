use std::collections::HashSet;

use itertools::Itertools;
use rltk::{DistanceAlg, Point, RandomNumberGenerator};

use crate::components::Position;
use crate::map::tiletype::TileType;
use crate::map_builders::{BuilderChain, BuilderMap, InitialMapBuilder};

pub fn town_builder(new_depth: i32, rng: &mut RandomNumberGenerator, width: i32, height: i32) -> BuilderChain {
    let mut chain = BuilderChain::new(new_depth, width, height);
    chain.start_with(TownBuilder::new());
    chain
}

pub struct TownBuilder {}

impl InitialMapBuilder for TownBuilder {
    fn build_map(&mut self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        self.build_rooms(rng, build_data);
    }
}

impl TownBuilder {
    pub fn new() -> Box<TownBuilder> {
        Box::new(Self {})
    }

    pub fn build_rooms(&mut self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        self.grass_layer(build_data);
        self.water_and_piers(rng, build_data);

        let (mut available_building_tiles, wall_gap_y) = self.town_walls(rng, build_data);
        let mut buildings = self.buildings(rng, build_data, &mut available_building_tiles);
        let doors = self.add_doors(rng, build_data, &mut buildings, wall_gap_y);
        self.add_paths(build_data, &doors);
        let exit_idx = build_data.map.xy_idx(build_data.width - 5, wall_gap_y);
        build_data.map.tiles[exit_idx] = TileType::DownStairs;

        let largest_index = buildings.iter().enumerate()
            .map(|(i, building)| (i, building.2 * building.3))
            .max_by(|a, b| a.1.cmp(&b.1))
            .map(|(i, _)| i)
            .unwrap();

        let the_pub = &buildings[largest_index];
        build_data.starting_position = Some(Position {
            x: the_pub.0 + (the_pub.2 / 2),
            y: the_pub.1 + (the_pub.3 / 2),
        });

        for t in build_data.map.visible_tiles.iter_mut() {
            *t = true;
        }

        build_data.take_snapshot();
    }
    fn grass_layer(&self, build_data: &mut BuilderMap) {
        for t in build_data.map.tiles.iter_mut() {
            *t = TileType::Grass;
        }
        build_data.take_snapshot();
    }
    fn water_and_piers(&self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        let mut n = (rng.roll_dice(1, 65535) as f32) / 65535f32;
        let mut water_width = Vec::new();

        for y in 0..build_data.height {
            let n_water = (f32::sin(n) * 10.0) as i32 + 14 + rng.roll_dice(1, 6);
            water_width.push(n_water);
            n += 0.1;
            for x in 0..n_water {
                let idx = build_data.map.xy_idx(x, y);
                build_data.map.tiles[idx] = TileType::DeepWater;
            }
            for x in n_water..n_water + 3 {
                let idx = build_data.map.xy_idx(x, y);
                build_data.map.tiles[idx] = TileType::ShallowWater;
            }
        }
        build_data.take_snapshot();

        for _i in 0..rng.roll_dice(1, 4) + 6 {
            let y = rng.roll_dice(1, build_data.height) - 1;
            for x in 2 + rng.roll_dice(1, 6)..water_width[y as usize] + 4 {
                let idx = build_data.map.xy_idx(x, y);
                build_data.map.tiles[idx] = TileType::WoodFloor;
            }
        }
        build_data.take_snapshot();
    }
    fn town_walls(&self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) -> (HashSet<usize>, i32) {
        let mut available_building_tiles = HashSet::new();
        let wall_gap_y = rng.roll_dice(1, build_data.height - 9) + 5;
        for y in 1..build_data.height - 2 {
            if !(y > wall_gap_y - 4 && y < wall_gap_y + 4) {
                let idx = build_data.map.xy_idx(30, y);
                build_data.map.tiles[idx] = TileType::Wall;
                build_data.map.tiles[idx - 1] = TileType::Floor;
                let idx_right = build_data.map.xy_idx(build_data.width - 2, y);
                build_data.map.tiles[idx_right] = TileType::Wall;
                for x in 31..build_data.width - 2 {
                    let gravel_idx = build_data.map.xy_idx(x, y);
                    build_data.map.tiles[gravel_idx] = TileType::Gravel;
                    if y > 2 && y < build_data.height - 1 {
                        available_building_tiles.insert(gravel_idx);
                    }
                }
            } else {
                for x in 30..build_data.width {
                    let road_idx = build_data.map.xy_idx(x, y);
                    build_data.map.tiles[road_idx] = TileType::Road;
                }
            }
        }
        build_data.take_snapshot();

        for x in 30..build_data.width - 1 {
            let idx_top = build_data.map.xy_idx(x, 1);
            build_data.map.tiles[idx_top] = TileType::Wall;
            let idx_bot = build_data.map.xy_idx(x, build_data.height - 2);
            build_data.map.tiles[idx_bot] = TileType::Wall;
        }
        build_data.take_snapshot();

        (available_building_tiles, wall_gap_y)
    }
    fn buildings(&self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap, available_building_tiles: &mut HashSet<usize>) -> Vec<(i32, i32, i32, i32)> {
        let mut buildings = Vec::new();
        let mut n_buildings = 0;
        while n_buildings < 12 {
            let bx = rng.roll_dice(1, build_data.map.width - 32) + 30;
            let by = rng.roll_dice(1, build_data.map.height) - 2;
            let bw = rng.roll_dice(1, 8) + 4;
            let bh = rng.roll_dice(1, 8) + 4;
            let mut possible = true;
            for y in by..by + bh {
                for x in bx..bx + bw {
                    if x < 0 || x > build_data.width - 1 || y < 0 || y > build_data.height - 1 {
                        possible = false;
                    } else {
                        let idx = build_data.map.xy_idx(x, y);
                        if !available_building_tiles.contains(&idx) { possible = false; }
                    }
                }
            }
            if possible {
                n_buildings += 1;
                buildings.push((bx, by, bw, bh));
                for y in by..by + bh {
                    for x in bx..bx + bw {
                        let idx = build_data.map.xy_idx(x, y);
                        build_data.map.tiles[idx] = TileType::WoodFloor;
                        available_building_tiles.remove(&idx);
                        available_building_tiles.remove(&(idx + 1));
                        available_building_tiles.remove(&(idx + build_data.width as usize));
                        available_building_tiles.remove(&(idx - 1));
                        available_building_tiles.remove(&(idx - build_data.width as usize));
                    }
                }
                build_data.take_snapshot();
            }
        }

        let mut mapclone = build_data.map.clone();
        for y in 2..build_data.height - 2 {
            for x in 32..build_data.width - 2 {
                let idx = build_data.map.xy_idx(x, y);
                if build_data.map.tiles[idx] == TileType::WoodFloor {
                    let mut neighbors = 0;
                    if build_data.map.tiles[idx - 1] != TileType::WoodFloor { neighbors += 1; }
                    if build_data.map.tiles[idx + 1] != TileType::WoodFloor { neighbors += 1; }
                    if build_data.map.tiles[idx - build_data.width as usize] != TileType::WoodFloor { neighbors += 1; }
                    if build_data.map.tiles[idx + build_data.width as usize] != TileType::WoodFloor { neighbors += 1; }
                    if neighbors > 0 {
                        mapclone.tiles[idx] = TileType::Wall;
                    }
                }
            }
        }

        build_data.map = mapclone;
        build_data.take_snapshot();
        buildings
    }
    fn add_doors(&self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap, buildings: &mut Vec<(i32, i32, i32, i32)>, wall_gap_y: i32) -> Vec<usize> {
        let mut doors = Vec::new();
        for building in buildings.iter() {
            let door_x = building.0 + 1 + rng.roll_dice(1, building.2 - 3);
            let cy = building.1 + (building.3 / 2);
            let idx = if cy > wall_gap_y {
                build_data.map.xy_idx(door_x, building.1)
            } else {
                build_data.map.xy_idx(door_x, building.1 + building.3 - 1)
            };
            build_data.map.tiles[idx] = TileType::Floor;
            build_data.spawn_list.push((idx, "Door".to_string()));
            doors.push(idx);
        }

        build_data.take_snapshot();
        doors
    }
    fn add_paths(&self, build_data: &mut BuilderMap, doors: &Vec<usize>) {
        let mut roads = Vec::new();
        for y in 0..build_data.height {
            for x in 0..build_data.width {
                let idx = build_data.map.xy_idx(x, y);
                if build_data.map.tiles[idx] == TileType::Road {
                    roads.push(idx);
                }
            }
        }

        build_data.map.populate_blocked();
        for door_idx in doors.iter() {
            let mut nearest_roads = Vec::new();
            let door_pt = rltk::Point::new(*door_idx as i32 % build_data.map.width as i32, *door_idx as i32 / build_data.map.width as i32);
            for r in roads.iter() {
                nearest_roads.push((
                    *r,
                    DistanceAlg::PythagorasSquared.distance2d(
                        door_pt,
                        Point::new(*r as i32 % build_data.map.width, *r as i32 / build_data.map.width),
                    )
                ));
            }
            nearest_roads.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            let destination = nearest_roads[0].0;
            let path = rltk::a_star_search(*door_idx, destination, &mut build_data.map);
            if path.success {
                for step in path.steps.iter() {
                    let idx = *step as usize;
                    build_data.map.tiles[idx] = TileType::Road;
                    roads.push(idx);
                }
            }
            build_data.take_snapshot();
        }
    }
}