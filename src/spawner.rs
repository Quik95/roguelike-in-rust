use std::collections::HashMap;

use rltk::{console, to_cp437, Point, BLACK, CYAN, RGB};
use specs::saveload::{MarkedBuilder, SimpleMarker};
use specs::{Builder, Entity, World, WorldExt};

use crate::components::{
    Attribute, AttributeBonus, Attributes, Duration, EntryTrigger, EquipmentChanged, Faction,
    HungerClock, HungerState, Initiative, KnownSpells, LightSource, Name, OtherLevelPosition,
    Player, Pool, Pools, Position, Renderable, SerializeMe, SingleActivation, Skill, Skills,
    StatusEffect, TeleportTo, Viewshed,
};
use crate::gamesystem::{attr_bonus, mana_at_level, player_hp_at_level};
use crate::map::dungeon::MasterDungeonMap;
use crate::map::{tiletype::TileType, Map};
use crate::random_table::MasterTable;
use crate::raws::rawmaster::{
    get_spawn_table_for_depth, spawn_all_spells, spawn_named_entity, SpawnType, RAWS,
};
use crate::rect::Rect;
use crate::rng::roll_dice;

const MAX_MONSTERS: i32 = 4;

pub fn player(ecs: &mut World, player_x: i32, player_y: i32) -> Entity {
    spawn_all_spells(ecs);

    let mut skills = Skills {
        skills: HashMap::new(),
    };
    skills.skills.insert(Skill::Melee, 1);
    skills.skills.insert(Skill::Defense, 1);
    skills.skills.insert(Skill::Magic, 1);

    let player = ecs
        .create_entity()
        .with(Position {
            x: player_x,
            y: player_y,
        })
        .with(Renderable {
            glyph: to_cp437('@'),
            fg: RGB::named(rltk::YELLOW),
            bg: RGB::named(rltk::BLACK),
            render_order: 0,
        })
        .with(Player {})
        .with(Viewshed {
            visible_tiles: Vec::new(),
            range: 8,
            dirty: true,
        })
        .with(Name {
            name: "Player".to_string(),
        })
        .with(HungerClock {
            state: HungerState::WellFed,
            duration: 20,
        })
        .with(Attributes {
            might: Attribute {
                base: 11,
                modifiers: 0,
                bonus: attr_bonus(11),
            },
            fitness: Attribute {
                base: 11,
                modifiers: 0,
                bonus: attr_bonus(11),
            },
            quickness: Attribute {
                base: 11,
                modifiers: 0,
                bonus: attr_bonus(11),
            },
            intelligence: Attribute {
                base: 11,
                modifiers: 0,
                bonus: attr_bonus(11),
            },
        })
        .with(skills)
        .with(Pools {
            hit_points: Pool {
                current: player_hp_at_level(11, 1),
                max: player_hp_at_level(11, 1),
            },
            mana: Pool {
                current: mana_at_level(11, 1),
                max: mana_at_level(11, 1),
            },
            xp: 0,
            level: 1,
            total_weight: 0.0,
            total_initiative_penalty: 0.0,
            gold: 0.0,
            god_mode: false,
        })
        .with(LightSource {
            color: RGB::from_f32(1.0, 1.0, 0.5),
            range: 8,
        })
        .with(Initiative { current: 0 })
        .with(EquipmentChanged {})
        .with(Faction {
            name: "Player".to_string(),
        })
        .with(KnownSpells { spells: vec![] })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();

    ecs.create_entity()
        .with(StatusEffect { target: player })
        .with(Duration { turns: 10 })
        .with(Name {
            name: "Hangover".into(),
        })
        .with(AttributeBonus {
            might: Some(-1),
            fitness: None,
            quickness: Some(-1),
            intelligence: Some(-1),
        })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();

    spawn_named_entity(
        &RAWS.lock().unwrap(),
        ecs,
        "Rusty Longsword",
        SpawnType::Equipped { by: player },
    );
    spawn_named_entity(
        &RAWS.lock().unwrap(),
        ecs,
        "Dried Sausage",
        SpawnType::Carried { by: player },
    );
    spawn_named_entity(
        &RAWS.lock().unwrap(),
        ecs,
        "Beer",
        SpawnType::Carried { by: player },
    );
    spawn_named_entity(
        &RAWS.lock().unwrap(),
        ecs,
        "Stained Tunic",
        SpawnType::Equipped { by: player },
    );
    spawn_named_entity(
        &RAWS.lock().unwrap(),
        ecs,
        "Torn Trousers",
        SpawnType::Equipped { by: player },
    );
    spawn_named_entity(
        &RAWS.lock().unwrap(),
        ecs,
        "Old Boots",
        SpawnType::Equipped { by: player },
    );
    spawn_named_entity(
        &RAWS.lock().unwrap(),
        ecs,
        "Shortbow",
        SpawnType::Carried { by: player },
    );

    player
}

pub fn spawn_room(map: &Map, room: &Rect, map_depth: i32, spawn_list: &mut Vec<(usize, String)>) {
    let mut possible_targets: Vec<usize> = Vec::new();
    {
        for y in room.y1 + 1..room.y2 {
            for x in room.x1 + 1..room.x2 {
                let idx = map.xy_idx(x, y);
                if map.tiles[idx] == TileType::Floor {
                    possible_targets.push(idx);
                }
            }
        }
    }

    spawn_region(map, &possible_targets, map_depth, spawn_list);
}

pub fn spawn_region(
    _map: &Map,
    area: &[usize],
    map_depth: i32,
    spawn_list: &mut Vec<(usize, String)>,
) {
    let spawn_table = room_table(map_depth);
    let mut spawn_points = HashMap::new();
    let mut areas = Vec::from(area);

    {
        let num_spawns = i32::min(
            areas.len() as i32,
            roll_dice(1, MAX_MONSTERS + 3) + (map_depth - 1) - 3,
        );
        if num_spawns == 0 {
            return;
        }

        for _i in 0..num_spawns {
            let array_index = if areas.len() == 1 {
                0usize
            } else {
                (roll_dice(1, areas.len() as i32) - 1) as usize
            };
            let map_idx = areas[array_index];
            spawn_points.insert(map_idx, spawn_table.roll());
            areas.remove(array_index);
        }
    }

    for spawn in &spawn_points {
        spawn_list.push((*spawn.0, spawn.1.to_string()));
    }
}

pub fn spawn_entity(ecs: &mut World, spawn: &(&usize, &String)) {
    let map = ecs.fetch::<Map>();
    let x = (*spawn.0 % map.width as usize) as i32;
    let y = (*spawn.0 / map.width as usize) as i32;
    std::mem::drop(map);

    let spawn_result = spawn_named_entity(
        &RAWS.lock().unwrap(),
        ecs,
        spawn.1,
        SpawnType::AtPosition { x, y },
    );
    if spawn_result.is_some() {
        return;
    }

    if spawn.1 != "None" {
        console::log(format!(
            "WARNING: We don't know how to spawn [{}]!",
            spawn.1
        ));
    }
}

fn room_table(map_depth: i32) -> MasterTable {
    get_spawn_table_for_depth(&RAWS.lock().unwrap(), map_depth)
}

pub fn spawn_town_portal(ecs: &mut World) {
    let map = ecs.fetch::<Map>();
    let player_depth = map.depth;
    let player_pos = ecs.fetch::<Point>();
    let player_x = player_pos.x;
    let player_y = player_pos.y;
    std::mem::drop(player_pos);
    std::mem::drop(map);

    let dm = ecs.fetch::<MasterDungeonMap>();
    let town_map = dm.get_map(1).unwrap();
    let mut stairs_idx = 0;
    for (idx, tt) in town_map.tiles.iter().enumerate() {
        if *tt == TileType::DownStairs {
            stairs_idx = idx;
        }
    }
    let portal_x = (stairs_idx as i32 % town_map.width) - 2;
    let portal_y = stairs_idx as i32 / town_map.width;

    std::mem::drop(dm);

    ecs.create_entity()
        .with(OtherLevelPosition {
            x: portal_x,
            y: portal_y,
            depth: 1,
        })
        .with(Renderable {
            glyph: to_cp437('♥'),
            fg: RGB::named(CYAN),
            bg: RGB::named(BLACK),
            render_order: 0,
        })
        .with(EntryTrigger {})
        .with(TeleportTo {
            x: player_x,
            y: player_y,
            depth: player_depth,
            player_only: true,
        })
        .with(Name {
            name: "Town Portal".into(),
        })
        .with(SingleActivation {})
        .build();
}
