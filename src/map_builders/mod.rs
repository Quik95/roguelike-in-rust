use rltk::RandomNumberGenerator;
use specs::World;

use crate::components::Position;
use crate::map::Map;
use crate::map_builders::bsp_dungeon::BspDungeonBuilder;
use crate::map_builders::bsp_interior::BspInteriorBuilder;
use crate::map_builders::cellular_automata::CellularAutomataBuilder;
use crate::map_builders::dla::DlaBuilder;
use crate::map_builders::drunkard::DrunkardsWalkBuilder;
use crate::map_builders::maze::MazeBuilder;
use crate::map_builders::prefab_builder::PrefabBuilder;
use crate::map_builders::simple_map::SimpleMapBuilder;
use crate::map_builders::voronoi::VoronoiCellBuilder;
use crate::map_builders::waveform_collapse::WaveformCollapseBuilder;
use crate::spawner;

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

pub trait MapBuilder {
    fn build_map(&mut self);
    fn get_map(&self) -> Map;
    fn get_starting_position(&mut self) -> Position;
    fn get_snapshot_history(&self) -> Vec<Map>;
    fn take_snapshot(&mut self);
    fn get_spawn_list(&self) -> &Vec<(usize, String)>;

    fn spawn_entities(&mut self, ecs: &mut World) {
        for entity in self.get_spawn_list().iter() {
            spawner::spawn_entity(ecs, &(&entity.0, &entity.1));
        }
    }
}

pub fn random_builder(new_depth: i32) -> Box<dyn MapBuilder> {
    let mut rng = RandomNumberGenerator::new();
    let builder = rng.roll_dice(1, 17);
    let mut result: Box<dyn MapBuilder> = match builder {
        1 => Box::new(SimpleMapBuilder::new(new_depth)),
        2 => Box::new(BspDungeonBuilder::new(new_depth)),
        3 => Box::new(BspInteriorBuilder::new(new_depth)),
        4 => Box::new(CellularAutomataBuilder::new(new_depth)),
        5 => Box::new(DrunkardsWalkBuilder::open_area(new_depth)),
        6 => Box::new(DrunkardsWalkBuilder::winding_passages(new_depth)),
        7 => Box::new(DrunkardsWalkBuilder::open_halls(new_depth)),
        8 => Box::new(MazeBuilder::new(new_depth)),
        9 => Box::new(DlaBuilder::walk_inwards(new_depth)),
        10 => Box::new(DlaBuilder::walk_outwards(new_depth)),
        11 => Box::new(DlaBuilder::central_attractor(new_depth)),
        12 => Box::new(DlaBuilder::insectoid(new_depth)),
        13 => Box::new(DrunkardsWalkBuilder::fat_passages(new_depth)),
        14 => Box::new(DrunkardsWalkBuilder::fearful_symmetry(new_depth)),
        15 => Box::new(VoronoiCellBuilder::pythagoras(new_depth)),
        16 => Box::new(VoronoiCellBuilder::manhattan(new_depth)),
        17 => Box::new(VoronoiCellBuilder::chebyshev(new_depth)),
        _ => unreachable!("Ups your forgot to add a builder."),
    };

    if rng.roll_dice(1, 3) == 1 {
        result = Box::new(WaveformCollapseBuilder::derived_map(new_depth, result));
    }

    result
}
