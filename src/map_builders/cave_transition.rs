use crate::map_builders::bsp_dungeon::BspDungeonBuilder;
use crate::map_builders::room_based_spawner::RoomBasedSpawner;
use crate::map_builders::room_draw::RoomDrawer;
use crate::map_builders::room_exploder::RoomExploder;
use crate::map_builders::room_sorter::{RoomSort, RoomSorter};
use crate::map_builders::rooms_corridors_nearest::NearestCorridors;
use crate::map_builders::{BuilderChain, BuilderMap, MetaMapBuilder};

pub struct CaveTransition {}

impl MetaMapBuilder for CaveTransition {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl CaveTransition {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn build(&mut self, build_data: &mut BuilderMap) {
        build_data.map.depth = 5;
        build_data.take_snapshot();

        let mut builder = BuilderChain::new(5, build_data.width, build_data.height, "New Map");
        builder.start_with(BspDungeonBuilder::new());
        builder.with(RoomDrawer::new());
        builder.with(RoomSorter::new(RoomSort::Rightmost));
        builder.with(NearestCorridors::new());
        builder.with(RoomExploder::new());
        builder.with(RoomBasedSpawner::new());
        builder.build_map();

        for h in &builder.build_data.history {
            build_data.history.push(h.clone());
        }
        build_data.take_snapshot();

        for x in build_data.map.width / 2..build_data.map.width {
            for y in 0..build_data.map.height {
                let idx = build_data.map.xy_idx(x, y);
                build_data.map.tiles[idx] = builder.build_data.map.tiles[idx];
            }
        }
        build_data.take_snapshot();

        let w = build_data.map.width;
        build_data.spawn_list.retain(|s| {
            let x = s.0 as i32 / w;
            x < w / 2
        });

        for s in &builder.build_data.spawn_list {
            let x = s.0 as i32 / w;
            if x > w / 2 {
                build_data.spawn_list.push(s.clone());
            }
        }
    }
}
