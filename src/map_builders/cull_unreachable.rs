use rltk::DijkstraMap;

use crate::map::tiletype::TileType;
use crate::map_builders::{BuilderMap, MetaMapBuilder};

pub struct CullUnreachable {}

impl MetaMapBuilder for CullUnreachable {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl CullUnreachable {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn build(&mut self, build_data: &mut BuilderMap) {
        let starting_pos = build_data.starting_position.as_ref().unwrap().clone();
        let start_idx = build_data.map.xy_idx(starting_pos.x, starting_pos.y);
        build_data.map.populate_blocked();
        let map_starts = vec![start_idx];
        let dijkstra_map = DijkstraMap::new(
            build_data.map.width as usize,
            build_data.map.height as usize,
            &map_starts,
            &build_data.map,
            1000.0,
        );
        for (i, tile) in build_data.map.tiles.iter_mut().enumerate() {
            if *tile == TileType::Floor {
                let distance_to_start = dijkstra_map.map[i];
                if distance_to_start == f32::MAX {
                    *tile = TileType::Wall;
                }
            }
        }
    }
}
