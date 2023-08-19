use rltk::{console, RandomNumberGenerator};
use specs::World;

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
use crate::map_builders::door_placement::DoorPlacement;
use crate::map_builders::drunkard::DrunkardsWalkBuilder;
use crate::map_builders::maze::MazeBuilder;
use crate::map_builders::prefab_builder::prefab_level::WFC_POPULATED;
use crate::map_builders::prefab_builder::prefab_section::UNDERGROUND_FORT;
use crate::map_builders::prefab_builder::PrefabBuilder;
use crate::map_builders::room_based_spawner::RoomBasedSpawner;
use crate::map_builders::room_based_stairs::RoomBasedStairs;
use crate::map_builders::room_based_starting_position::RoomBasedStartingPosition;
use crate::map_builders::room_corner_rounding::RoomCornerRounder;
use crate::map_builders::room_corridor_spawner::CorridorSpawner;
use crate::map_builders::room_draw::RoomDrawer;
use crate::map_builders::room_exploder::RoomExploder;
use crate::map_builders::room_sorter::{RoomSort, RoomSorter};
use crate::map_builders::rooms_corridors_bsp::BspCorridors;
use crate::map_builders::rooms_corridors_dogleg::DoglegCorridors;
use crate::map_builders::simple_map::SimpleMapBuilder;
use crate::map_builders::town::town_builder;
use crate::map_builders::voronoi::VoronoiCellBuilder;
use crate::map_builders::voronoi_spawning::VoronoiSpawning;
use crate::map_builders::waveform_collapse::WaveformCollapseBuilder;
use crate::rect::Rect;

mod area_starting_points;
mod bsp_dungeon;
mod bsp_interior;
mod cellular_automata;
mod common;
mod cull_unreachable;
mod distant_exit;
mod dla;
mod door_placement;
mod drunkard;
mod maze;
mod prefab_builder;
mod room_based_spawner;
mod room_based_stairs;
mod room_based_starting_position;
mod room_corner_rounding;
mod room_corridor_spawner;
mod room_draw;
mod room_exploder;
mod room_sorter;
mod rooms_corridors_bsp;
mod rooms_corridors_dogleg;
mod rooms_corridors_lines;
mod rooms_corridors_nearest;
mod simple_map;
mod voronoi;
mod voronoi_spawning;
mod waveform_collapse;
mod town;

#[derive(Default)]
pub struct BuilderMap {
    pub spawn_list: Vec<(usize, String)>,
    pub map: Map,
    pub starting_position: Option<Position>,
    pub rooms: Option<Vec<Rect>>,
    pub history: Vec<Map>,
    pub corridors: Option<Vec<Vec<usize>>>,
    pub width: i32,
    pub height: i32,
}

pub fn level_builder(new_depth: i32, rng: &mut RandomNumberGenerator, width: i32, height: i32) -> BuilderChain {
    console::log(format!("Depth: {new_depth}"));
    match new_depth {
        1 => town_builder(new_depth, rng, width, height),
        _ => random_builder(new_depth, rng, width, height)
    }
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
    pub fn new(new_depth: i32, width: i32, height: i32) -> Self {
        Self {
            starter: None,
            builders: Vec::new(),
            build_data: BuilderMap {
                spawn_list: Vec::new(),
                map: Map::new(new_depth, width, height),
                starting_position: None,
                rooms: None,
                history: Vec::new(),
                corridors: None,
                width,
                height,
            },
        }
    }

    pub fn start_with(&mut self, starter: Box<dyn InitialMapBuilder>) {
        match self.starter {
            None => self.starter = Some(starter),
            Some(_) => panic!("You can only have one starting builder."),
        }
    }

    pub fn with(&mut self, metabuilder: Box<dyn MetaMapBuilder>) {
        self.builders.push(metabuilder);
    }

    pub fn build_map(&mut self, rng: &mut RandomNumberGenerator) {
        match &mut self.starter {
            None => panic!("Cannot run a map builder chain without a starting build system"),
            Some(starter) => starter.build_map(rng, &mut self.build_data),
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

pub fn random_builder(new_depth: i32, rng: &mut RandomNumberGenerator, width: i32, height: i32) -> BuilderChain {
    let mut builder = BuilderChain::new(new_depth, width, height);
    let type_roll = rng.roll_dice(1, 2);
    match type_roll {
        1 => random_room_builder(rng, &mut builder),
        2 => random_shape_builder(rng, &mut builder),
        _ => unreachable!(),
    }

    if rng.roll_dice(1, 3) == 1 {
        builder.with(WaveformCollapseBuilder::new());

        let (start_x, start_y) = random_start_position(rng);
        builder.with(AreaStartingPosition::new(start_x, start_y));

        builder.with(VoronoiSpawning::new());
        builder.with(DistantExit::new());
    }

    if rng.roll_dice(1, 20) == 1 {
        builder.with(PrefabBuilder::sectional(UNDERGROUND_FORT));
    }

    builder.with(DoorPlacement::new());
    builder.with(PrefabBuilder::vaults());

    builder
}

fn random_room_builder(rng: &mut RandomNumberGenerator, builder: &mut BuilderChain) {
    let build_roll = rng.roll_dice(1, 3);
    match build_roll {
        1 => builder.start_with(SimpleMapBuilder::new()),
        2 => builder.start_with(BspDungeonBuilder::new()),
        3 => builder.start_with(BspInteriorBuilder::new()),
        _ => unreachable!(),
    }

    if build_roll != 3 {
        let sort_roll = rng.roll_dice(1, 5);
        match sort_roll {
            1 => builder.with(RoomSorter::new(RoomSort::Leftmost)),
            2 => builder.with(RoomSorter::new(RoomSort::Rightmost)),
            3 => builder.with(RoomSorter::new(RoomSort::Topmost)),
            4 => builder.with(RoomSorter::new(RoomSort::Bottommost)),
            5 => builder.with(RoomSorter::new(RoomSort::Central)),
            _ => unreachable!(),
        }

        builder.with(RoomDrawer::new());

        let corridor_roll = rng.roll_dice(1, 2);
        match corridor_roll {
            1 => builder.with(DoglegCorridors::new()),
            2 => builder.with(BspCorridors::new()),
            _ => unreachable!(),
        }

        let cspawn_roll = rng.roll_dice(1, 2);
        if cspawn_roll == 1 {
            builder.with(CorridorSpawner::new());
        }

        let modifier_roll = rng.roll_dice(1, 6);
        match modifier_roll {
            1 => builder.with(RoomExploder::new()),
            2 => builder.with(RoomCornerRounder::new()),
            _ => {}
        }
    }

    let start_roll = rng.roll_dice(1, 2);
    match start_roll {
        1 => builder.with(RoomBasedStartingPosition::new()),
        2 => {
            let (start_x, start_y) = random_start_position(rng);
            builder.with(AreaStartingPosition::new(start_x, start_y));
        }
        _ => unreachable!(),
    }

    let exit_roll = rng.roll_dice(1, 2);
    match exit_roll {
        1 => builder.with(RoomBasedStairs::new()),
        2 => builder.with(DistantExit::new()),
        _ => unreachable!(),
    }

    let spawn_roll = rng.roll_dice(1, 1);
    match spawn_roll {
        1 => builder.with(RoomBasedSpawner::new()),
        2 => builder.with(VoronoiSpawning::new()),
        _ => unreachable!(),
    }
}

fn random_shape_builder(rng: &mut RandomNumberGenerator, builder: &mut BuilderChain) {
    let builder_roll = rng.roll_dice(1, 18);
    match builder_roll {
        1 => builder.start_with(CellularAutomataBuilder::new()),
        2 => builder.start_with(DrunkardsWalkBuilder::open_area()),
        3 => builder.start_with(DrunkardsWalkBuilder::open_halls()),
        4 => builder.start_with(DrunkardsWalkBuilder::winding_passages()),
        5 => builder.start_with(DrunkardsWalkBuilder::fat_passages()),
        6 => builder.start_with(DrunkardsWalkBuilder::fearful_symmetry()),
        7 => builder.start_with(MazeBuilder::new()),
        8 => builder.start_with(DlaBuilder::walk_inwards()),
        9 => builder.start_with(DlaBuilder::walk_outwards()),
        10 => builder.start_with(DlaBuilder::central_attractor()),
        11 => builder.start_with(DlaBuilder::insectoid()),
        12 => builder.start_with(DlaBuilder::heavy_erosion()),
        13 => builder.start_with(VoronoiCellBuilder::pythagoras()),
        14 => builder.start_with(VoronoiCellBuilder::manhattan()),
        15 => builder.start_with(VoronoiCellBuilder::chebyshev()),
        _ => builder.start_with(PrefabBuilder::constant(WFC_POPULATED)),
    }

    builder.with(AreaStartingPosition::new(XStart::Center, YStart::Center));
    builder.with(CullUnreachable::new());

    let (start_x, start_y) = random_start_position(rng);
    builder.with(AreaStartingPosition::new(start_x, start_y));

    builder.with(VoronoiSpawning::new());
    builder.with(DistantExit::new());
}

fn random_start_position(rng: &mut RandomNumberGenerator) -> (XStart, YStart) {
    let start_x = match rng.roll_dice(1, 3) {
        1 => XStart::Left,
        2 => XStart::Center,
        3 => XStart::Right,
        _ => unreachable!(),
    };

    let start_y = match rng.roll_dice(1, 3) {
        1 => YStart::Bottom,
        2 => YStart::Center,
        3 => YStart::Top,
        _ => unreachable!(),
    };

    (start_x, start_y)
}

