use rltk::RandomNumberGenerator;

use crate::cave_decorator::CaveDecorator;
use crate::map_builders::area_starting_points::{AreaStartingPosition, XStart, YStart};
use crate::map_builders::cull_unreachable::CullUnreachable;
use crate::map_builders::distant_exit::DistantExit;
use crate::map_builders::drunkard::DrunkardsWalkBuilder;
use crate::map_builders::voronoi_spawning::VoronoiSpawning;
use crate::map_builders::BuilderChain;

pub fn limestone_cavern_builder(
    new_depth: i32,
    _rng: &mut RandomNumberGenerator,
    width: i32,
    height: i32,
) -> BuilderChain {
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
