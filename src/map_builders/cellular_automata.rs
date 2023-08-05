use std::collections::{hash_map, HashMap};
use std::process::id;
use std::thread::spawn;
use rltk::{CellularDistanceFunction, DijkstraMap, FastNoise, NoiseType, RandomNumberGenerator};
use specs::World;

use crate::components::Position;
use crate::map::{Map, TileType};
use crate::map_builders::MapBuilder;
use crate::{SHOW_MAPGEN_VISUALIZER, spawner};

const MIN_ROOM_SIZE: i32 = 8;

pub struct CellularAutomataBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    noise_areas: HashMap<i32, Vec<usize>>
}

impl MapBuilder for CellularAutomataBuilder {
    fn build_map(&mut self) {
        self.build()
    }

    fn spawn_entities(&mut self, ecs: &mut World) {
        for area in self.noise_areas.iter() {
            spawner::spawn_region(ecs, area.1, self.depth);
        }
    }

    fn get_map(&self) -> Map {
        self.map.clone()
    }

    fn get_starting_position(&mut self) -> Position {
        self.starting_position.clone()
    }

    fn get_snapshot_history(&self) -> Vec<Map> {
        self.history.clone()
    }

    fn take_snapshot(&mut self) {
        if SHOW_MAPGEN_VISUALIZER {
            let mut snapshot = self.map.clone();
            for v in snapshot.revealed_tiles.iter_mut() {
                *v = true;
            }
            self.history.push(snapshot);
        }
    }
}

impl CellularAutomataBuilder {
    pub fn new(new_depth: i32) -> Self {
        Self {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new()
        }
    }

    fn build(&mut self) {
        let mut rng = RandomNumberGenerator::new();

        for y in 1..self.map.height - 1 {
            for x in 1..self.map.width - 1 {
                let roll = rng.roll_dice(1, 100);
                let idx = Map::xy_idx(x, y);
                if roll > 55 { self.map.tiles[idx] = TileType::Floor } else { self.map.tiles[idx] = TileType::Wall }
            }
        }
        self.take_snapshot();

        for _i in 0..15 {
            let mut newtiles = self.map.tiles.clone();

            for y in 1..self.map.height - 1 {
                for x in 1..self.map.width - 1 {
                    let idx = Map::xy_idx(x, y);
                    let mut neighbors = 0;
                    if self.map.tiles[idx - 1] == TileType::Wall { neighbors += 1; }
                    if self.map.tiles[idx + 1] == TileType::Wall { neighbors += 1; }
                    if self.map.tiles[idx - self.map.width as usize] == TileType::Wall { neighbors += 1; }
                    if self.map.tiles[idx + self.map.width as usize] == TileType::Wall { neighbors += 1; }
                    if self.map.tiles[idx - (self.map.width as usize - 1)] == TileType::Wall { neighbors += 1; }
                    if self.map.tiles[idx - (self.map.width as usize + 1)] == TileType::Wall { neighbors += 1; }
                    if self.map.tiles[idx + (self.map.width as usize - 1)] == TileType::Wall { neighbors += 1; }
                    if self.map.tiles[idx + (self.map.width as usize + 1)] == TileType::Wall { neighbors += 1; }

                    if neighbors > 4 || neighbors == 0 {
                        newtiles[idx] = TileType::Wall;
                    } else {
                        newtiles[idx] = TileType::Floor;
                    }
                }
            }
            self.map.tiles = newtiles.clone();
            self.take_snapshot();
        }

        self.starting_position = Position { x: self.map.width / 2, y: self.map.height / 2 };
        let mut start_idx = Map::xy_idx(self.starting_position.x, self.starting_position.y);
        while self.map.tiles[start_idx] != TileType::Floor {
            self.starting_position.x -= 1;
            start_idx = Map::xy_idx(self.starting_position.x, self.starting_position.y);
        }

        let map_start = vec![start_idx];
        let dijkstra_map = DijkstraMap::new(self.map.width, self.map.height, &map_start, &self.map, 200.0);
        let mut exit_tile = (0, 0.0f32);
        for (i, tile) in self.map.tiles.iter_mut().enumerate() {
            if *tile == TileType::Floor {
                let distance_to_start = dijkstra_map.map[i];
                if distance_to_start == f32::MAX {
                    *tile = TileType::Wall;
                } else if distance_to_start > exit_tile.1   {
                    exit_tile.0 = i;
                    exit_tile.1 = distance_to_start;
                }
            }
        }
        self.take_snapshot();

        self.map.tiles[exit_tile.0] = TileType::DownStairs;
        self.take_snapshot();

        let mut noise = FastNoise::seeded(rng.roll_dice(1, 65536i32) as u64);
        noise.set_noise_type(NoiseType::Cellular);
        noise.set_frequency(0.08);
        noise.set_cellular_distance_function(CellularDistanceFunction::Manhattan);

        for y in 1..self.map.height-1 {
            for x in 1..self.map.width-1 {
                let idx = Map::xy_idx(x, y);
                if self.map.tiles[idx] == TileType::Floor {
                    let cell_value_f = noise.get_noise(x as f32, y as f32) * 10240.0;
                    let cell_value = cell_value_f as i32;

                    if let hash_map::Entry::Vacant(e) = self.noise_areas.entry(cell_value) {
                        e.insert(vec![idx]);
                    } else {
                        self.noise_areas.get_mut(&cell_value).unwrap().push(idx);
                    }
                }
            }
        }
    }
}