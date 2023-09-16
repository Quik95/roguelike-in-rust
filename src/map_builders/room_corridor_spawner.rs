use crate::map_builders::{BuilderMap, MetaMapBuilder};
use crate::spawner;

pub struct CorridorSpawner {}

impl MetaMapBuilder for CorridorSpawner {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl CorridorSpawner {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn build(&mut self, build_data: &mut BuilderMap) {
        if let Some(corridors) = &build_data.corridors {
            for c in corridors {
                let depth = build_data.map.depth;
                spawner::spawn_region(&build_data.map, c, depth, &mut build_data.spawn_list);
            }
        } else {
            panic!("Corridor Based Spawning only workds after corridors have been created.");
        }
    }
}
