use std::sync::Mutex;

use itertools::Itertools;
use lazy_static::lazy_static;
use specs::Entity;

use crate::map::Map;
use crate::player::RunState;

#[derive(Default, Debug, Clone)]
struct SpatialMap {
    blocked: Vec<(bool, bool)>,
    tile_content: Vec<Vec<(Entity, bool)>>,
}

lazy_static! {
    static ref SPATIAL_MAP: Mutex<SpatialMap> = Mutex::new(SpatialMap::default());
}

pub fn set_size(map_tile_count: usize) {
    let mut lock = SPATIAL_MAP.lock().unwrap();
    lock.blocked = vec![(false, false); map_tile_count];
    lock.tile_content = vec![Vec::new(); map_tile_count];
}

pub fn clear() {
    let mut lock = SPATIAL_MAP.lock().unwrap();
    lock.blocked.iter_mut().for_each(|b| {
        b.0 = false;
        b.1 = false;
    });
    for content in &mut lock.tile_content {
        content.clear();
    }
}

pub fn set_blocked(idx: usize, blocked: bool) {
    let mut lock = SPATIAL_MAP.lock().unwrap();
    lock.blocked[idx] = (lock.blocked[idx].0, blocked);
}

pub fn populate_blocked_from_map(map: &Map) {
    let mut lock = SPATIAL_MAP.lock().unwrap();
    for (i, tile) in map.tiles.iter().enumerate() {
        lock.blocked[i].0 = !tile.is_walkable();
    }
}

pub fn index_entity(entity: Entity, idx: usize, blocks_tile: bool) {
    let mut lock = SPATIAL_MAP.lock().unwrap();
    lock.tile_content[idx].push((entity, blocks_tile));
    if blocks_tile {
        lock.blocked[idx].1 = true;
    }
}

pub fn get_tile_content_clone(idx: usize) -> Vec<Entity> {
    let lock = SPATIAL_MAP.lock().unwrap();
    lock.tile_content[idx].iter().map(|(e, _)| *e).collect_vec()
}

pub fn is_blocked(idx: usize) -> bool {
    let lock = SPATIAL_MAP.lock().unwrap();
    lock.blocked[idx].0 || lock.blocked[idx].1
}

pub fn for_each_tile_content(idx: usize, mut f: impl FnMut(Entity)) {
    let lock = SPATIAL_MAP.lock().unwrap();
    for entity in &lock.tile_content[idx] {
        f(entity.0);
    }
}

pub fn move_entity(entity: Entity, moving_from: usize, moving_to: usize) {
    let mut lock = SPATIAL_MAP.lock().unwrap();
    let mut entity_blocks = false;
    lock.tile_content[moving_from].retain(|(e, blocks)| {
        if *e == entity {
            entity_blocks = *blocks;
            false
        } else {
            true
        }
    });
    lock.tile_content[moving_from].push((entity, entity_blocks));

    let mut from_blocked = false;
    let mut to_blocked = false;
    lock.tile_content[moving_from]
        .iter()
        .for_each(|(_, blocks)| {
            if *blocks {
                from_blocked = true;
            }
        });
    lock.tile_content[moving_to]
        .iter_mut()
        .for_each(|(_, blocks)| {
            if *blocks {
                to_blocked = true;
            }
        });
    lock.blocked[moving_from].1 = from_blocked;
    lock.blocked[moving_to].1 = to_blocked;
}

pub fn for_each_tile_content_with_gamemode(
    idx: usize,
    mut f: impl FnMut(Entity) -> Option<RunState>,
) -> RunState {
    let lock = SPATIAL_MAP.lock().unwrap();
    for entity in &lock.tile_content[idx] {
        if let Some(rs) = f(entity.0) {
            return rs;
        }
    }

    RunState::AwaitingInput
}

pub fn remove_entity(entity: Entity, idx: usize) {
    let mut lock = SPATIAL_MAP.lock().unwrap();
    lock.tile_content[idx].retain(|(e, _)| *e != entity);
    let mut from_blocked = false;
    lock.tile_content[idx].iter().for_each(|(_, blocks)| {
        if *blocks {
            from_blocked = true;
        }
    });
    lock.blocked[idx].1 = from_blocked;
}
