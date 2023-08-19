use serde::{Deserialize, Serialize};
use TileType::*;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum TileType {
    Wall,
    Floor,
    DownStairs,
    Road,
    Grass,
    ShallowWater,
    DeepWater,
    WoodFloor,
    Bridge,
    Gravel
}

impl TileType {
    pub fn is_walkable(&self) -> bool {
        match self {
            Floor | DownStairs | Road | Grass | ShallowWater | WoodFloor | Bridge | Gravel => true,
            Wall | DeepWater => false
        }
    }

    pub fn is_opaque(&self) -> bool {
        match self {
            Wall => true,
            _ => false
        }
    }

    pub fn get_cost(&self) -> f32 {
        match self {
            Road => 0.8,
            Grass => 1.1,
            ShallowWater => 1.2,
            _ => 1.0
        }
    }
}