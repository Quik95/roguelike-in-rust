use rltk::RandomNumberGenerator;
use specs::{Builder, World};

use crate::{SHOW_MAPGEN_VISUALIZER, spawner};
use crate::components::Position;
use crate::map::Map;
use crate::map_builders::area_starting_points::{AreaStartingPosition, XStart, YStart};
use crate::map_builders::bsp_dungeon::BspDungeonBuilder;
use crate::map_builders::bsp_interior::BspInteriorBuilder;
use crate::map_builders::cellular_automata::CellularAutomataBuilder;
use crate::map_builders::cull_unreachable::CullUnreachable;
use crate::map_builders::distant_exit::DistantExit;
use crate::map_builders::dla::DlaBuilder;
use crate::map_builders::drunkard::DrunkardsWalkBuilder;
use crate::map_builders::maze::MazeBuilder;
use crate::map_builders::prefab_builder::prefab_level::WFC_POPULATED;
use crate::map_builders::prefab_builder::prefab_section::UNDERGROUND_FORT;
use crate::map_builders::prefab_builder::PrefabBuilder;
use crate::map_builders::room_based_spawner::RoomBasedSpawner;
use crate::map_builders::room_based_stairs::RoomBasedStairs;
use crate::map_builders::room_based_starting_position::RoomBasedStartingPosition;
use crate::map_builders::simple_map::SimpleMapBuilder;
use crate::map_builders::voronoi::VoronoiCellBuilder;
use crate::map_builders::voronoi_spawning::VoronoiSpawning;
use crate::map_builders::waveform_collapse::WaveformCollapseBuilder;
use crate::rect::Rect;

mod bsp_dungeon;
mod bsp_interior;
mod cellular_automata;
mod common;
mod dla;
mod drunkard;
mod maze;
mod prefab_builder;
mod simple_map;
mod voronoi;
mod waveform_collapse;
mod room_based_spawner;
mod room_based_starting_position;
mod room_based_stairs;
mod area_starting_points;
mod cull_unreachable;
mod voronoi_spawning;
mod distant_exit;

#[derive(Default)]
pub struct BuilderMap {
    pub spawn_list: Vec<(usize, String)>,
    pub map: Map,
    pub starting_position: Option<Position>,
    pub rooms: Option<Vec<Rect>>,
    pub history: Vec<Map>,
}

impl BuilderMap {
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

pub trait InitialMapBuilder {
    fn build_map(&mut self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap);
}

pub trait MetaMapBuilder {
    fn build_map(&mut self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap);
}

#[derive(Default)]
pub struct BuilderChain {
    starter: Option<Box<dyn InitialMapBuilder>>,
    builders: Vec<Box<dyn MetaMapBuilder>>,
    pub build_data: BuilderMap,
}

impl BuilderChain {
    pub fn new(new_depth: i32) -> Self {
        Self {
            starter: None,
            builders: Vec::new(),
            build_data: BuilderMap {
                spawn_list: Vec::new(),
                map: Map::new(new_depth),
                starting_position: None,
                rooms: None,
                history: Vec::new(),
            },
        }
    }

    pub fn start_with(&mut self, starter: Box<dyn InitialMapBuilder>) {
        match self.starter {
            None => self.starter = Some(starter),
            Some(_) => panic!("You can only have one starting builder.")
        }
    }

    pub fn with(&mut self, metabuilder: Box<dyn MetaMapBuilder>) {
        self.builders.push(metabuilder);
    }

    pub fn build_map(&mut self, rng: &mut RandomNumberGenerator) {
        match &mut self.starter {
            None => panic!("Cannot run a map builder chain without a starting build system"),
            Some(starter) => starter.build_map(rng, &mut self.build_data)
        }

        for metabuilder in self.builders.iter_mut() {
            metabuilder.build_map(rng, &mut self.build_data);
        }
    }

    pub fn spawn_entities(&mut self, ecs: &mut World) {
        for entity in self.build_data.spawn_list.iter() {
            spawner::spawn_entity(ecs, &(&entity.0, &entity.1));
        }
    }
}

pub fn random_builder(new_depth: i32, rng: &mut RandomNumberGenerator) -> BuilderChain {
    let mut builder = BuilderChain::new(new_depth);
    let (random_starter, has_rooms) = random_initial_builder(rng);
    builder.start_with(random_starter);
    if has_rooms {
        builder.with(RoomBasedSpawner::new());
        builder.with(RoomBasedStairs::new());
        builder.with(RoomBasedStartingPosition::new());
    } else {
        builder.with(AreaStartingPosition::new(XStart::CENTER, YStart::CENTER));
        builder.with(CullUnreachable::new());
        builder.with(VoronoiSpawning::new());
        builder.with(DistantExit::new());
    }

    if rng.roll_dice(1, 3) == 1 {
        builder.with(WaveformCollapseBuilder::new());
    }

    if rng.roll_dice(1, 20) == 1 {
        builder.with(PrefabBuilder::sectional(UNDERGROUND_FORT));
    }

    builder.with(PrefabBuilder::vaults());

    builder
}

pub fn random_initial_builder(rng: &mut RandomNumberGenerator) -> (Box<dyn InitialMapBuilder>, bool) {
    let builder = rng.roll_dice(1, 18);
    let result: (Box<dyn InitialMapBuilder>, bool) = match builder {
        1 => (SimpleMapBuilder::new(), true),
        2 => (BspDungeonBuilder::new(), true),
        3 => (BspInteriorBuilder::new(), true),
        4 => (CellularAutomataBuilder::new(), false),
        5 => (DrunkardsWalkBuilder::open_area(), false),
        6 => (DrunkardsWalkBuilder::winding_passages(), false),
        7 => (DrunkardsWalkBuilder::open_halls(), false),
        8 => (DrunkardsWalkBuilder::fat_passages(), false),
        9 => (DrunkardsWalkBuilder::fearful_symmetry(), false),
        10 => (MazeBuilder::new(), false),
        11 => (DlaBuilder::walk_inwards(), false),
        12 => (DlaBuilder::walk_outwards(), false),
        13 => (DlaBuilder::central_attractor(), false),
        14 => (DlaBuilder::insectoid(), false),
        15 => (VoronoiCellBuilder::pythagoras(), false),
        16 => (VoronoiCellBuilder::manhattan(), false),
        17 => (VoronoiCellBuilder::chebyshev(), false),
        18 => (PrefabBuilder::constant(WFC_POPULATED), false),
        _ => unreachable!("Ups your forgot to add a builder."),
    };

    result
}