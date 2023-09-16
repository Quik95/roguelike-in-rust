use crate::map_builders::area_starting_points::{
    AreaEndingPosition, AreaStartingPosition, XEnd, XStart, YEnd, YStart,
};
use crate::map_builders::bsp_interior::BspInteriorBuilder;
use crate::map_builders::cull_unreachable::CullUnreachable;
use crate::map_builders::voronoi_spawning::VoronoiSpawning;
use crate::map_builders::BuilderChain;

pub fn dark_elf_city(new_depth: i32, width: i32, height: i32) -> BuilderChain {
    let mut chain = BuilderChain::new(new_depth, width, height, "Dark Elven City");
    chain.start_with(BspInteriorBuilder::new());
    chain.with(AreaStartingPosition::new(XStart::Center, YStart::Center));
    chain.with(CullUnreachable::new());
    chain.with(AreaStartingPosition::new(XStart::Right, YStart::Center));
    chain.with(AreaEndingPosition::new(XEnd::Left, YEnd::Center));
    chain.with(VoronoiSpawning::new());

    chain
}
