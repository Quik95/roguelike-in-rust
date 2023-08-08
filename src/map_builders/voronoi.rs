use std::collections::HashMap;

use rltk::{DistanceAlg, Point, RandomNumberGenerator};
use specs::World;

use crate::{SHOW_MAPGEN_VISUALIZER, spawner};
use crate::components::Position;
use crate::map::{Map, TileType};
use crate::map_builders::common::{generate_voronoi_spawn_regions, remove_unreachable_areas_returning_most_distant};
use crate::map_builders::MapBuilder;

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum DistanceAlgorithm { Pythagoras, Manhattan, Chebyshev }

pub struct VoronoiCellBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    noise_areas: HashMap<i32, Vec<usize>>,
    n_seeds: usize,
    distance_algorithm: DistanceAlgorithm,
    spawn_list: Vec<(usize, String)>
}

impl MapBuilder for VoronoiCellBuilder {
    fn build_map(&mut self) {
        self.build()
    }

    fn get_spawn_list(&self) -> &Vec<(usize, String)> {
        &self.spawn_list
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

impl VoronoiCellBuilder {
    fn new(new_depth: i32) -> Self {
        Self {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
            n_seeds: 64,
            distance_algorithm: DistanceAlgorithm::Pythagoras,
            spawn_list: Vec::new()
        }
    }

    pub fn pythagoras(new_depth: i32) -> Self {
        Self {
            distance_algorithm: DistanceAlgorithm::Pythagoras,
            ..Self::new(new_depth)
        }
    }

    pub fn manhattan(new_depth: i32) -> Self {
        Self {
            distance_algorithm: DistanceAlgorithm::Manhattan,
            ..Self::new(new_depth)
        }
    }

    pub fn chebyshev(new_depth: i32) -> Self {
        Self {
            distance_algorithm: DistanceAlgorithm::Chebyshev,
            ..Self::new(new_depth)
        }
    }

    fn build(&mut self) {
        let mut rng = RandomNumberGenerator::new();

        let mut voronoi_seeds = Vec::new();

        while voronoi_seeds.len() < self.n_seeds {
            let vx = rng.roll_dice(1, self.map.width - 1);
            let vy = rng.roll_dice(1, self.map.height - 1);
            let vidx = Map::xy_idx(vx, vy);
            let candidate = (vidx, Point::new(vx, vy));
            if !voronoi_seeds.contains(&candidate) {
                voronoi_seeds.push(candidate);
            }
        }

        let mut voronoi_distance = vec![(0, 0.0f32); self.n_seeds];
        let mut voronoi_membership = vec![0; self.map.width as usize * self.map.height as usize];
        for (i, vid) in voronoi_membership.iter_mut().enumerate() {
            let x = i as i32 % self.map.width;
            let y = i as i32 / self.map.width;

            for (seed, pos) in voronoi_seeds.iter().enumerate() {
                let distance = match self.distance_algorithm {
                    DistanceAlgorithm::Pythagoras =>
                        DistanceAlg::PythagorasSquared.distance2d(Point::new(x, y), pos.1),
                    DistanceAlgorithm::Manhattan =>
                        DistanceAlg::Manhattan.distance2d(Point::new(x, y), pos.1),
                    DistanceAlgorithm::Chebyshev =>
                        DistanceAlg::Chebyshev.distance2d(Point::new(x, y), pos.1)
                };
                voronoi_distance[seed] = (seed, distance);
            }

            voronoi_distance.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            *vid = voronoi_distance[0].0 as i32;
        }

        for y in 1..self.map.height - 1 {
            for x in 1..self.map.width - 1 {
                let mut neighbors = 0;
                let my_idx = Map::xy_idx(x, y);
                let my_seed = voronoi_membership[my_idx];
                if voronoi_membership[Map::xy_idx(x - 1, y)] != my_seed { neighbors += 1; }
                if voronoi_membership[Map::xy_idx(x + 1, y)] != my_seed { neighbors += 1; }
                if voronoi_membership[Map::xy_idx(x, y - 1)] != my_seed { neighbors += 1; }
                if voronoi_membership[Map::xy_idx(x, y + 1)] != my_seed { neighbors += 1; }

                if neighbors < 2 {
                    self.map.tiles[my_idx] = TileType::Floor;
                }
            }
            self.take_snapshot();
        }

        self.starting_position = Position { x: self.map.width / 2, y: self.map.height / 2 };
        let mut start_idx = Map::xy_idx(self.starting_position.x, self.starting_position.y);
        while self.map.tiles[start_idx] != TileType::Floor {
            self.starting_position.x -= 1;
            start_idx = Map::xy_idx(self.starting_position.x, self.starting_position.y);
        }
        self.take_snapshot();

        let exit_tile = remove_unreachable_areas_returning_most_distant(&mut self.map, start_idx);
        self.take_snapshot();

        self.map.tiles[exit_tile] = TileType::DownStairs;
        self.take_snapshot();

        self.noise_areas = generate_voronoi_spawn_regions(&self.map, &mut rng);

        for area in self.noise_areas.iter() {
            spawner::spawn_region(&self.map, &mut rng, area.1, self.depth, &mut self.spawn_list);
        }
    }
}
