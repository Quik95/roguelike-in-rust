use crate::cave_decorator::CaveDecorator;
use crate::map_builders::area_starting_points::{
    AreaEndingPosition, AreaStartingPosition, XEnd, XStart, YEnd, YStart,
};
use crate::map_builders::cave_transition::CaveTransition;
use crate::map_builders::cellular_automata::CellularAutomataBuilder;
use crate::map_builders::cull_unreachable::CullUnreachable;
use crate::map_builders::distant_exit::DistantExit;
use crate::map_builders::dla::DlaBuilder;
use crate::map_builders::drunkard::DrunkardsWalkBuilder;
use crate::map_builders::prefab_builder::{prefab_section, PrefabBuilder};
use crate::map_builders::voronoi_spawning::VoronoiSpawning;
use crate::map_builders::BuilderChain;

pub fn limestone_cavern_builder(new_depth: i32, width: i32, height: i32) -> BuilderChain {
    let mut chain = BuilderChain::new(new_depth, width, height, "Limestone Caverns");
    chain.start_with(DrunkardsWalkBuilder::winding_passages());
    chain.with(AreaStartingPosition::new(XStart::Center, YStart::Center));
    chain.with(CullUnreachable::new());
    chain.with(AreaStartingPosition::new(XStart::Left, YStart::Center));
    chain.with(VoronoiSpawning::new());
    chain.with(DistantExit::new());
    chain.with(CaveDecorator::new());
    chain
}

pub fn limestone_deep_cavern_builder(new_depth: i32, width: i32, height: i32) -> BuilderChain {
    let mut chain = BuilderChain::new(new_depth, width, height, "Deep Limestone Caverns");
    chain.start_with(DlaBuilder::central_attractor());
    chain.with(AreaStartingPosition::new(XStart::Left, YStart::Top));
    chain.with(VoronoiSpawning::new());
    chain.with(DistantExit::new());
    chain.with(CaveDecorator::new());
    chain.with(PrefabBuilder::sectional(prefab_section::ORC_CAMP));

    chain
}

pub fn limestone_transition_builder(new_depth: i32, width: i32, height: i32) -> BuilderChain {
    let mut chain = BuilderChain::new(new_depth, width, height, "Dwarf Fort - Upper Reaches");
    chain.start_with(CellularAutomataBuilder::new());
    chain.with(AreaStartingPosition::new(XStart::Center, YStart::Center));
    chain.with(CullUnreachable::new());
    chain.with(AreaStartingPosition::new(XStart::Left, YStart::Center));
    chain.with(VoronoiSpawning::new());
    chain.with(CaveDecorator::new());
    chain.with(CaveTransition::new());
    chain.with(AreaStartingPosition::new(XStart::Left, YStart::Center));
    chain.with(CullUnreachable::new());
    chain.with(AreaEndingPosition::new(XEnd::Right, YEnd::Center));

    chain
}
