use serde::{Deserialize, Serialize};

use TileType::{
    Bridge, DeepWater, DownStairs, Floor, Grass, Gravel, Road, ShallowWater, Stalactite,
    Stalagmite, UpStairs, Wall, WoodFloor,
};

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
    Gravel,
    UpStairs,
    Stalactite,
    Stalagmite,
}

impl TileType {
    pub const fn is_walkable(&self) -> bool {
        match self {
            Floor | DownStairs | Road | Grass | ShallowWater | WoodFloor | Bridge | Gravel
            | UpStairs => true,
            Wall | DeepWater | Stalactite | Stalagmite => false,
        }
    }

    pub const fn is_opaque(&self) -> bool {
        matches!(self, Wall | Stalactite | Stalagmite)
    }

    pub const fn get_cost(&self) -> f32 {
        match self {
            Road => 0.8,
            Grass => 1.1,
            ShallowWater => 1.2,
            _ => 1.0,
        }
    }
}
