use rltk::RandomNumberGenerator;

use crate::map::TileType;

use super::{BuilderMap, MetaMapBuilder};

pub struct DoorPlacement {}

impl MetaMapBuilder for DoorPlacement {
    fn build_map(
        &mut self,
        rng: &mut rltk::RandomNumberGenerator,
        build_data: &mut super::BuilderMap,
    ) {
        self.doors(rng, build_data);
    }
}

impl DoorPlacement {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn doors(&mut self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        if let Some(halls_original) = &build_data.corridors {
            let halls = halls_original.clone();
            for hall in halls.iter() {
                if hall.len() > 2 && self.door_possible(build_data, hall[0]) {
                    build_data.spawn_list.push((hall[0], "Door".to_string()));
                }
            }
        } else {
            let tiles = build_data.map.tiles.clone();
            for (i, tile) in tiles.iter().enumerate() {
                if *tile == TileType::Floor && self.door_possible(build_data, i) && rng.roll_dice(1, 3) == 1 {
                    build_data.spawn_list.push((i, "Door".to_string()));
                }
            }
        }
    }

    fn door_possible(&self, build_data: &mut BuilderMap, idx: usize) -> bool {
        let mut blocked = false;
        for spawn in build_data.spawn_list.iter() {
            if spawn.0 == idx { blocked = true; }
        }
        if blocked { return false; }

        let x = idx % build_data.map.width as usize;
        let y = idx / build_data.map.height as usize;

        // Check for east-west door possibility
        if build_data.map.tiles[idx] == TileType::Floor
            && (x > 1 && build_data.map.tiles[idx - 1] == TileType::Floor)
            && (x < build_data.map.width as usize - 2
            && build_data.map.tiles[idx + 1] == TileType::Floor)
            && (y > 1
            && build_data.map.tiles[idx - build_data.map.width as usize] == TileType::Wall)
            && (y < build_data.map.height as usize - 2
            && build_data.map.tiles[idx + build_data.map.width as usize] == TileType::Wall)
        {
            return true;
        }

        // Check for north-south door possibility
        if build_data.map.tiles[idx] == TileType::Floor
            && (x > 1 && build_data.map.tiles[idx - 1] == TileType::Wall)
            && (x < build_data.map.width as usize - 2
            && build_data.map.tiles[idx + 1] == TileType::Wall)
            && (y > 1
            && build_data.map.tiles[idx - build_data.map.width as usize] == TileType::Floor)
            && (y < build_data.map.height as usize - 2
            && build_data.map.tiles[idx + build_data.map.width as usize] == TileType::Floor)
        {
            return true;
        }

        false
    }
}
