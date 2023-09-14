use rltk::{DistanceAlg, Point, RandomNumberGenerator};

use crate::map::tiletype::TileType;
use crate::map_builders::area_starting_points::{
    AreaEndingPosition, AreaStartingPosition, XEnd, XStart, YEnd, YStart,
};
use crate::map_builders::bsp_dungeon::BspDungeonBuilder;
use crate::map_builders::cull_unreachable::CullUnreachable;
use crate::map_builders::distant_exit::DistantExit;
use crate::map_builders::dla::DlaBuilder;
use crate::map_builders::room_corridor_spawner::CorridorSpawner;
use crate::map_builders::room_draw::RoomDrawer;
use crate::map_builders::room_sorter::{RoomSort, RoomSorter};
use crate::map_builders::rooms_corridors_bsp::BspCorridors;
use crate::map_builders::voronoi_spawning::VoronoiSpawning;
use crate::map_builders::{BuilderChain, BuilderMap, MetaMapBuilder};

pub fn dwarf_fort_builder(
    new_depth: i32,
    _rng: &mut RandomNumberGenerator,
    width: i32,
    height: i32,
) -> BuilderChain {
    let mut chain = BuilderChain::new(new_depth, width, height, "Dwarven Fortress");
    chain.start_with(BspDungeonBuilder::new());
    chain.with(RoomSorter::new(RoomSort::Central));
    chain.with(RoomDrawer::new());
    chain.with(BspCorridors::new());
    chain.with(CorridorSpawner::new());
    chain.with(DragonsLair::new());
    chain.with(DragonSpawner::new());

    chain.with(AreaStartingPosition::new(XStart::Left, YStart::Top));
    chain.with(CullUnreachable::new());
    chain.with(AreaEndingPosition::new(XEnd::Right, YEnd::Bottom));
    chain.with(VoronoiSpawning::new());
    chain.with(DistantExit::new());

    chain
}

pub struct DragonsLair {}

impl MetaMapBuilder for DragonsLair {
    fn build_map(&mut self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        self.build(rng, build_data)
    }
}

impl DragonsLair {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn build(&mut self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        build_data.map.depth = 6;
        build_data.take_snapshot();

        let mut builder = BuilderChain::new(6, build_data.width, build_data.height, "New Map");
        builder.start_with(DlaBuilder::insectoid());
        builder.build_map(rng);

        for h in builder.build_data.history.iter() {
            build_data.history.push(h.clone());
        }
        build_data.take_snapshot();

        for (idx, tt) in build_data.map.tiles.iter_mut().enumerate() {
            if *tt == TileType::Wall && builder.build_data.map.tiles[idx] == TileType::Floor {
                *tt = TileType::Floor;
            }
        }
        build_data.take_snapshot();
    }
}

pub struct DragonSpawner {}

impl MetaMapBuilder for DragonSpawner {
    fn build_map(&mut self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        self.build(rng, build_data);
    }
}

impl DragonSpawner {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn build(&mut self, _rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        let seed_x = build_data.map.width / 2;
        let seed_y = build_data.map.height / 2;
        let mut available_floors = vec![];
        for (idx, tiletyle) in build_data.map.tiles.iter().enumerate() {
            if !tiletyle.is_walkable() {
                continue;
            }

            available_floors.push((
                idx,
                DistanceAlg::PythagorasSquared.distance2d(
                    Point::new(
                        idx as i32 % build_data.map.width,
                        idx as i32 / build_data.map.width,
                    ),
                    Point::new(seed_x, seed_y),
                ),
            ));
        }
        if available_floors.is_empty() {
            panic!("No valid floors to start on");
        }

        available_floors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let start_x = available_floors[0].0 as i32 % build_data.map.width;
        let start_y = available_floors[0].0 as i32 / build_data.map.width;
        let dragon_pt = Point::new(start_x, start_y);

        let w = build_data.map.width as i32;
        build_data.spawn_list.retain(|spawn| {
            let spawn_pt = Point::new(spawn.0 as i32 % w, spawn.0 as i32 / w);
            let distance = DistanceAlg::Pythagoras.distance2d(dragon_pt, spawn_pt);
            distance > 25.0
        });

        let dragon_idx = build_data.map.xy_idx(start_x, start_y);
        build_data
            .spawn_list
            .push((dragon_idx, "Black Dragon".into()));
    }
}
