use crate::map_builders::common::Symmetry::Horizontal;

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum HorizontalPlacement{Left, Center, Right}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum VerticalPlacement{Top, Center, Bottom}

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct PrefabSection {
    pub template: &'static str,
    pub width: usize,
    pub height: usize,
    pub placement: (HorizontalPlacement, VerticalPlacement)
}

pub const UNDERGROUND_FORT: PrefabSection = PrefabSection{
    template: RIGHT_FORT,
    width: 14,
    height: 43,
    placement: (HorizontalPlacement::Right, VerticalPlacement::Top)
};

const RIGHT_FORT : &str = "
  ######
  ## ###
    ^
    ^
  ## ###
  ## ###
    ^
    ^
  ## ###
  ######
";