use core::default::Default;
use std::collections::{HashMap, HashSet};

use rltk::Point;
use serde::{Deserialize, Serialize};
use specs::{Entity, Join, World, WorldExt};

use crate::components::{OtherLevelPosition, Position, Viewshed};
use crate::map::tiletype::TileType;
use crate::map::Map;
use crate::map_builders::level_builder;
use crate::raws::rawmaster::{get_potion_tag, get_scroll_tags};
use crate::rng::roll_dice;

#[derive(Serialize, Deserialize, Clone)]
pub struct MasterDungeonMap {
    pub maps: HashMap<i32, Map>,
    pub identified_items: HashSet<String>,
    pub scroll_mappings: HashMap<String, String>,
    pub potion_mappings: HashMap<String, String>,
}

impl Default for MasterDungeonMap {
    fn default() -> Self {
        let mut dm = Self {
            maps: Default::default(),
            identified_items: Default::default(),
            scroll_mappings: Default::default(),
            potion_mappings: Default::default(),
        };

        for scroll_tag in &get_scroll_tags() {
            let masked_name = make_scroll_name();
            dm.scroll_mappings.insert(scroll_tag.into(), masked_name);
        }

        let mut used_potion_names = HashSet::new();
        for potion_tag in &get_potion_tag() {
            let masked_name = make_potion_name(&mut used_potion_names);
            dm.potion_mappings
                .insert(potion_tag.to_string(), masked_name);
        }

        dm
    }
}

impl MasterDungeonMap {
    pub fn store(&mut self, map: &Map) {
        self.maps.insert(map.depth, map.clone());
    }

    pub fn get_map(&self, depth: i32) -> Option<Map> {
        if self.maps.contains_key(&depth) {
            let result = self.maps[&depth].clone();
            Some(result)
        } else {
            None
        }
    }
}

fn make_scroll_name() -> String {
    let length = 4 + roll_dice(1, 4);
    let mut name = "Scroll of ".into();

    for i in 0..length {
        if i % 2 == 0 {
            name += match roll_dice(1, 5) {
                1 => "a",
                2 => "e",
                3 => "i",
                4 => "o",
                5 => "u",
                _ => unreachable!(),
            }
        } else {
            name += match roll_dice(1, 21) {
                1 => "b",
                2 => "c",
                3 => "d",
                4 => "f",
                5 => "g",
                6 => "h",
                7 => "j",
                8 => "k",
                9 => "l",
                10 => "m",
                11 => "n",
                12 => "p",
                13 => "q",
                14 => "r",
                15 => "s",
                16 => "t",
                17 => "v",
                18 => "w",
                19 => "x",
                20 => "y",
                21 => "z",
                _ => unreachable!(),
            }
        }
    }

    name
}

const POTION_COLORS: &[&str] = &[
    "Red", "Orange", "Yellow", "Green", "Brown", "Indigo", "Violet",
];
const POTION_ADJECTIVES: &[&str] = &[
    "Swirling",
    "Effervescent",
    "Slimey",
    "Oiley",
    "Viscous",
    "Smelly",
    "Glowing",
];

fn make_potion_name(used_names: &mut HashSet<String>) -> String {
    loop {
        let mut name = POTION_ADJECTIVES[roll_dice(1, POTION_ADJECTIVES.len() as i32) as usize - 1]
            .to_string();
        name += " ";
        name += POTION_COLORS[roll_dice(1, POTION_COLORS.len() as i32) as usize - 1];
        name += " Potion";

        if !used_names.contains(&name) {
            used_names.insert(name.clone());
            return name;
        }
    }
}

pub fn level_transition(ecs: &mut World, new_depth: i32, offset: i32) -> Option<Vec<Map>> {
    let dungeon_master = ecs.read_resource::<MasterDungeonMap>();

    if dungeon_master.get_map(new_depth).is_some() {
        std::mem::drop(dungeon_master);
        transition_to_existing_map(ecs, new_depth, offset);
        None
    } else {
        std::mem::drop(dungeon_master);
        Some(transition_to_new_map(ecs, new_depth))
    }
}

fn transition_to_existing_map(ecs: &World, new_depth: i32, offset: i32) {
    let dungeon_master = ecs.read_resource::<MasterDungeonMap>();
    let map = dungeon_master.get_map(new_depth).unwrap();
    let mut worldmap_resource = ecs.write_resource::<Map>();
    let player_entity = ecs.fetch::<Entity>();

    // Find the down stairs and place the player
    let w = map.width;
    let stair_type = if offset < 0 {
        TileType::DownStairs
    } else {
        TileType::UpStairs
    };
    for (idx, tt) in map.tiles.iter().enumerate() {
        if *tt == stair_type {
            let mut player_position = ecs.write_resource::<Point>();
            *player_position = Point::new(idx as i32 % w, idx as i32 / w);
            let mut position_components = ecs.write_storage::<Position>();
            let player_pos_comp = position_components.get_mut(*player_entity);
            if let Some(player_pos_comp) = player_pos_comp {
                player_pos_comp.x = idx as i32 % w;
                player_pos_comp.y = idx as i32 / w;
            }
        }
    }

    *worldmap_resource = map;

    // Mark the player's visibility as dirty
    let mut viewshed_components = ecs.write_storage::<Viewshed>();
    let vs = viewshed_components.get_mut(*player_entity);
    if let Some(vs) = vs {
        vs.dirty = true;
    }
}

fn transition_to_new_map(ecs: &mut World, new_depth: i32) -> Vec<Map> {
    let mut builder = level_builder(new_depth, 80, 50);
    builder.build_map();
    if new_depth > 1 {
        if let Some(pos) = &builder.build_data.starting_position {
            let up_idx = builder.build_data.map.xy_idx(pos.x, pos.y);
            builder.build_data.map.tiles[up_idx] = TileType::UpStairs;
        }
    }
    let mapgen_history = builder.build_data.history.clone();
    let player_start;
    {
        let mut worldmap_resource = ecs.write_resource::<Map>();
        *worldmap_resource = builder.build_data.map.clone();
        player_start = builder
            .build_data
            .starting_position
            .as_mut()
            .unwrap()
            .clone();
    }

    // Spawn bad guys
    builder.spawn_entities(ecs);

    // Place the player and update resources
    let (player_x, player_y) = (player_start.x, player_start.y);
    let mut player_position = ecs.write_resource::<Point>();
    *player_position = Point::new(player_x, player_y);
    let mut position_components = ecs.write_storage::<Position>();
    let player_entity = ecs.fetch::<Entity>();
    let player_pos_comp = position_components.get_mut(*player_entity);
    if let Some(player_pos_comp) = player_pos_comp {
        player_pos_comp.x = player_x;
        player_pos_comp.y = player_y;
    }

    // Mark the player's visibility as dirty
    let mut viewshed_components = ecs.write_storage::<Viewshed>();
    let vs = viewshed_components.get_mut(*player_entity);
    if let Some(vs) = vs {
        vs.dirty = true;
    }

    // Store the newly minted map
    let mut dungeon_master = ecs.write_resource::<MasterDungeonMap>();
    dungeon_master.store(&builder.build_data.map);

    mapgen_history
}

pub fn freeze_level_entities(ecs: &World) {
    // Obtain ECS access
    let entities = ecs.entities();
    let mut positions = ecs.write_storage::<Position>();
    let mut other_level_positions = ecs.write_storage::<OtherLevelPosition>();
    let player_entity = ecs.fetch::<Entity>();
    let map_depth = ecs.fetch::<Map>().depth;

    // Find positions and make OtherLevelPosition
    let mut pos_to_delete: Vec<Entity> = Vec::new();
    for (entity, pos) in (&entities, &positions).join() {
        if entity != *player_entity {
            other_level_positions
                .insert(
                    entity,
                    OtherLevelPosition {
                        x: pos.x,
                        y: pos.y,
                        depth: map_depth,
                    },
                )
                .expect("Insert fail");
            pos_to_delete.push(entity);
        }
    }

    // Remove positions
    for p in &pos_to_delete {
        positions.remove(*p);
    }
}

pub fn thaw_level_entities(ecs: &World) {
    // Obtain ECS access
    let entities = ecs.entities();
    let mut positions = ecs.write_storage::<Position>();
    let mut other_level_positions = ecs.write_storage::<OtherLevelPosition>();
    let player_entity = ecs.fetch::<Entity>();
    let map_depth = ecs.fetch::<Map>().depth;

    // Find OtherLevelPosition
    let mut pos_to_delete: Vec<Entity> = Vec::new();
    for (entity, pos) in (&entities, &other_level_positions).join() {
        if entity != *player_entity && pos.depth == map_depth {
            positions
                .insert(entity, Position { x: pos.x, y: pos.y })
                .expect("Insert fail");
            pos_to_delete.push(entity);
        }
    }

    // Remove positions
    for p in &pos_to_delete {
        other_level_positions.remove(*p);
    }
}
