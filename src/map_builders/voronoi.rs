use rltk::{DistanceAlg, Point};

use crate::map::tiletype::TileType;
use crate::map_builders::{BuilderMap, InitialMapBuilder};
use crate::rng::roll_dice;

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum DistanceAlgorithm {
    Pythagoras,
    Manhattan,
    Chebyshev,
}

pub struct VoronoiCellBuilder {
    n_seeds: usize,
    distance_algorithm: DistanceAlgorithm,
}

impl InitialMapBuilder for VoronoiCellBuilder {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl VoronoiCellBuilder {
    const fn new() -> Self {
        Self {
            n_seeds: 64,
            distance_algorithm: DistanceAlgorithm::Pythagoras,
        }
    }

    pub fn pythagoras() -> Box<Self> {
        Box::new(Self {
            distance_algorithm: DistanceAlgorithm::Pythagoras,
            ..Self::new()
        })
    }

    pub fn manhattan() -> Box<Self> {
        Box::new(Self {
            distance_algorithm: DistanceAlgorithm::Manhattan,
            ..Self::new()
        })
    }

    pub fn chebyshev() -> Box<Self> {
        Box::new(Self {
            distance_algorithm: DistanceAlgorithm::Chebyshev,
            ..Self::new()
        })
    }

    fn build(&mut self, build_data: &mut BuilderMap) {
        let mut voronoi_seeds = Vec::new();

        while voronoi_seeds.len() < self.n_seeds {
            let vx = roll_dice(1, build_data.map.width - 1);
            let vy = roll_dice(1, build_data.map.height - 1);
            let vidx = build_data.map.xy_idx(vx, vy);
            let candidate = (vidx, Point::new(vx, vy));
            if !voronoi_seeds.contains(&candidate) {
                voronoi_seeds.push(candidate);
            }
        }

        let mut voronoi_distance = vec![(0, 0.0f32); self.n_seeds];
        let mut voronoi_membership =
            vec![0; build_data.map.width as usize * build_data.map.height as usize];
        for (i, vid) in voronoi_membership.iter_mut().enumerate() {
            let x = i as i32 % build_data.map.width;
            let y = i as i32 / build_data.map.width;

            for (seed, pos) in voronoi_seeds.iter().enumerate() {
                let distance = match self.distance_algorithm {
                    DistanceAlgorithm::Pythagoras => {
                        DistanceAlg::PythagorasSquared.distance2d(Point::new(x, y), pos.1)
                    }
                    DistanceAlgorithm::Manhattan => {
                        DistanceAlg::Manhattan.distance2d(Point::new(x, y), pos.1)
                    }
                    DistanceAlgorithm::Chebyshev => {
                        DistanceAlg::Chebyshev.distance2d(Point::new(x, y), pos.1)
                    }
                };
                voronoi_distance[seed] = (seed, distance);
            }

            voronoi_distance.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            *vid = voronoi_distance[0].0 as i32;
        }

        for y in 1..build_data.map.height - 1 {
            for x in 1..build_data.map.width - 1 {
                let mut neighbors = 0;
                let my_idx = build_data.map.xy_idx(x, y);
                let my_seed = voronoi_membership[my_idx];
                if voronoi_membership[build_data.map.xy_idx(x - 1, y)] != my_seed {
                    neighbors += 1;
                }
                if voronoi_membership[build_data.map.xy_idx(x + 1, y)] != my_seed {
                    neighbors += 1;
                }
                if voronoi_membership[build_data.map.xy_idx(x, y - 1)] != my_seed {
                    neighbors += 1;
                }
                if voronoi_membership[build_data.map.xy_idx(x, y + 1)] != my_seed {
                    neighbors += 1;
                }

                if neighbors < 2 {
                    build_data.map.tiles[my_idx] = TileType::Floor;
                }
            }
            build_data.take_snapshot();
        }
    }
}
