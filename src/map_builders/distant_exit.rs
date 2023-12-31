use rltk::DijkstraMap;

use crate::map::tiletype::TileType;
use crate::map_builders::{BuilderMap, MetaMapBuilder};

pub struct DistantExit {}

impl MetaMapBuilder for DistantExit {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl DistantExit {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn build(&mut self, build_data: &mut BuilderMap) {
        let starting_pos = build_data.starting_position.as_ref().unwrap().clone();
        let start_idx = build_data.map.xy_idx(starting_pos.x, starting_pos.y);
        build_data.map.populate_blocked();
        let map_start = vec![start_idx];
        let dijkstra_map = DijkstraMap::new(
            build_data.map.width as usize,
            build_data.map.height as usize,
            &map_start,
            &build_data.map,
            1000.0,
        );
        let mut exit_tile = (0, 0f32);
        for (i, tile) in build_data.map.tiles.iter_mut().enumerate() {
            if *tile == TileType::Floor {
                let distance_to_start = dijkstra_map.map[i];
                if distance_to_start != f32::MAX && distance_to_start > exit_tile.1 {
                    exit_tile.0 = i;
                    exit_tile.1 = distance_to_start;
                }
            }
        }

        let stairs_idx = exit_tile.0;
        build_data.map.tiles[stairs_idx] = TileType::DownStairs;
        build_data.take_snapshot();
    }
}
