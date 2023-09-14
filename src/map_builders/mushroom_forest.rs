use crate::map_builders::area_starting_points::{
    AreaEndingPosition, AreaStartingPosition, XEnd, XStart, YEnd, YStart,
};
use crate::map_builders::cellular_automata::CellularAutomataBuilder;
use crate::map_builders::cull_unreachable::CullUnreachable;
use crate::map_builders::prefab_builder::prefab_section::UNDERGROUND_FORT;
use crate::map_builders::prefab_builder::PrefabBuilder;
use crate::map_builders::voronoi_spawning::VoronoiSpawning;
use crate::map_builders::waveform_collapse::WaveformCollapseBuilder;
use crate::map_builders::BuilderChain;
use rltk::RandomNumberGenerator;

pub fn mushroom_entrance(
    new_depth: i32,
    _rng: &mut RandomNumberGenerator,
    width: i32,
    height: i32,
) -> BuilderChain {
    let mut chain = BuilderChain::new(new_depth, width, height, "Into the Mushroom Grove");
    chain.start_with(CellularAutomataBuilder::new());
    chain.with(WaveformCollapseBuilder::new());
    chain.with(AreaStartingPosition::new(XStart::Center, YStart::Center));
    chain.with(CullUnreachable::new());
    chain.with(AreaStartingPosition::new(XStart::Right, YStart::Center));
    chain.with(AreaEndingPosition::new(XEnd::Left, YEnd::Center));
    chain.with(VoronoiSpawning::new());
    chain.with(PrefabBuilder::sectional(UNDERGROUND_FORT));

    chain
}
