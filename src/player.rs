use rltk::{to_cp437, Point, RandomNumberGenerator, VirtualKeyCode};
use specs::prelude::*;

use crate::components::{KnownSpells, WantsToCastSpell};
use crate::gamelog::GameLog;
use crate::map::tiletype::TileType;
use crate::player::RunState::{
    NextLevel, PreviousLevel, SaveGame, ShowCheatMenu, ShowDropItem, ShowInventory, ShowRemoveItem,
    Ticking,
};
use crate::raws::rawmaster::{faction_reaction, find_spell_entity, RAWS};
use crate::raws::Reaction;
use crate::{gui, spatial};

use super::components::{
    Attributes, BlocksTile, BlocksVisibility, Consumable, Door, EntityMoved, Faction, HungerClock,
    HungerState, InBackpack, Item, Player, Pools, Position, Ranged, Renderable, Vendor, Viewshed,
    WantsToMelee, WantsToPickupItem, WantsToUseItem,
};
use super::map::Map;
use super::State;

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum RunState {
    AwaitingInput,
    PreRun,
    Ticking,
    ShowInventory,
    ShowDropItem,
    ShowTargeting {
        range: i32,
        item: Entity,
    },
    MainMenu {
        menu_selection: gui::MainMenuSelection,
    },
    SaveGame,
    NextLevel,
    PreviousLevel,
    ShowRemoveItem,
    GameOver,
    MagicMapReveal {
        row: i32,
    },
    MapGeneration,
    ShowCheatMenu,
    ShowVendor {
        vendor: Entity,
        mode: VendorMode,
    },
    TownPortal,
    TeleportingToOtherLevel {
        x: i32,
        y: i32,
        depth: i32,
    },
    ShowRemoveCurse,
    ShowIdentify,
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum VendorMode {
    Buy,
    Sell,
}

impl Player {
    pub fn try_move_player(delta_x: i32, delta_y: i32, ecs: &World) -> RunState {
        let mut positions = ecs.write_storage::<Position>();
        let mut players = ecs.write_storage::<Self>();
        let mut viewsheds = ecs.write_storage::<Viewshed>();
        let entities = ecs.entities();
        let combat_stats = ecs.read_storage::<Attributes>();
        let map = ecs.fetch::<Map>();
        let mut wants_to_melee = ecs.write_storage::<WantsToMelee>();
        let mut entity_moved = ecs.write_storage::<EntityMoved>();
        let mut doors = ecs.write_storage::<Door>();
        let mut blocks_visibility = ecs.write_storage::<BlocksVisibility>();
        let mut blocks_movement = ecs.write_storage::<BlocksTile>();
        let mut renderables = ecs.write_storage::<Renderable>();
        let factions = ecs.read_storage::<Faction>();
        let mut ppos = ecs.write_resource::<Point>();
        let vendors = ecs.read_storage::<Vendor>();

        let mut result = RunState::AwaitingInput;
        let mut swap_entities = Vec::new();

        for (entity, _player, pos, viewshed) in
            (&entities, &mut players, &mut positions, &mut viewsheds).join()
        {
            if pos.x + delta_x < 1
                || pos.x + delta_x > map.width - 1
                || pos.y + delta_y < 1
                || pos.y + delta_y > map.height - 1
            {
                return RunState::AwaitingInput;
            }
            let destination_idx = map.xy_idx(pos.x + delta_x, pos.y + delta_y);

            result =
                spatial::for_each_tile_content_with_gamemode(destination_idx, |potential_target| {
                    if let Some(_vendor) = vendors.get(potential_target) {
                        return Some(RunState::ShowVendor {
                            vendor: potential_target,
                            mode: VendorMode::Sell,
                        });
                    }

                    let mut hostile = true;
                    if combat_stats.get(potential_target).is_some() {
                        if let Some(faction) = factions.get(potential_target) {
                            let reaction =
                                faction_reaction(&faction.name, "Player", &RAWS.lock().unwrap());
                            if reaction != Reaction::Attack {
                                hostile = false;
                            }
                        }
                    }
                    if !hostile {
                        swap_entities.push((potential_target, pos.x, pos.y));

                        pos.x = (pos.x + delta_x).clamp(0, map.width - 1);
                        pos.x = (pos.y + delta_y).clamp(0, map.height - 1);
                        entity_moved
                            .insert(entity, EntityMoved {})
                            .expect("Unable to insert marker");

                        viewshed.dirty = true;
                        ppos.x = pos.x;
                        ppos.y = pos.y;
                        return Some(RunState::Ticking);
                    } else {
                        let target = combat_stats.get(potential_target);
                        if let Some(_target) = target {
                            wants_to_melee
                                .insert(
                                    entity,
                                    WantsToMelee {
                                        target: potential_target,
                                    },
                                )
                                .expect("Add target failed");
                            return Some(RunState::Ticking);
                        }
                    }
                    let door = doors.get_mut(potential_target);
                    if let Some(door) = door {
                        door.open = true;
                        blocks_visibility.remove(potential_target);
                        blocks_movement.remove(potential_target);
                        let glyph = renderables.get_mut(potential_target).unwrap();
                        glyph.glyph = to_cp437('/');
                        viewshed.dirty = true;
                        return Some(RunState::Ticking);
                    }
                    None
                });

            if !spatial::is_blocked(destination_idx) {
                let old_idx = map.xy_idx(pos.x, pos.y);
                pos.x = i32::min(map.width - 1, i32::max(0, pos.x + delta_x));
                pos.y = i32::min(map.height - 1, i32::max(0, pos.y + delta_y));
                let new_idx = map.xy_idx(pos.x, pos.y);
                entity_moved
                    .insert(entity, EntityMoved {})
                    .expect("Unable to insert marker");
                spatial::move_entity(entity, old_idx, new_idx);

                viewshed.dirty = true;
                ppos.x = pos.x;
                ppos.y = pos.y;
                result = RunState::Ticking;
                match map.tiles[destination_idx] {
                    TileType::DownStairs => result = RunState::NextLevel,
                    TileType::UpStairs => result = RunState::PreviousLevel,
                    _ => {}
                }
            }
        }

        for m in &swap_entities {
            let their_pos = positions.get_mut(m.0);
            if let Some(their_pos) = their_pos {
                let old_idx = map.xy_idx(their_pos.x, their_pos.y);
                their_pos.x = m.1;
                their_pos.y = m.2;
                let new_idx = map.xy_idx(their_pos.x, their_pos.y);
                spatial::move_entity(m.0, old_idx, new_idx);
                result = RunState::Ticking;
            }
        }

        result
    }

    pub fn player_input(gs: &State, ctx: &mut rltk::Rltk) -> RunState {
        // TODO: fix this
        if ctx.shift && ctx.key.is_some() {
            let key: Option<i32> = match ctx.key.unwrap() {
                VirtualKeyCode::Key1 => Some(1),
                VirtualKeyCode::Key2 => Some(2),
                VirtualKeyCode::Key3 => Some(3),
                VirtualKeyCode::Key4 => Some(4),
                VirtualKeyCode::Key5 => Some(5),
                VirtualKeyCode::Key6 => Some(6),
                VirtualKeyCode::Key7 => Some(7),
                VirtualKeyCode::Key8 => Some(8),
                VirtualKeyCode::Key9 => Some(9),
                _ => None,
            };
            if let Some(key) = key {
                return use_consumable_hotkey(gs, key - 1);
            }
        }

        if ctx.control && ctx.key.is_some() {
            let key: Option<i32> = match ctx.key.unwrap() {
                VirtualKeyCode::Key1 => Some(1),
                VirtualKeyCode::Key2 => Some(2),
                VirtualKeyCode::Key3 => Some(3),
                VirtualKeyCode::Key4 => Some(4),
                VirtualKeyCode::Key5 => Some(5),
                VirtualKeyCode::Key6 => Some(6),
                VirtualKeyCode::Key7 => Some(7),
                VirtualKeyCode::Key8 => Some(8),
                VirtualKeyCode::Key9 => Some(9),
                _ => None,
            };
            if let Some(key) = key {
                return use_spell_hotkey(gs, key - 1);
            }
        }

        match ctx.key {
            None => return RunState::AwaitingInput,
            Some(key) => match key {
                VirtualKeyCode::Left | VirtualKeyCode::Numpad4 | VirtualKeyCode::H => {
                    return Self::try_move_player(-1, 0, &gs.ecs)
                }
                VirtualKeyCode::Right | VirtualKeyCode::Numpad6 | VirtualKeyCode::L => {
                    return Self::try_move_player(1, 0, &gs.ecs)
                }
                VirtualKeyCode::Up | VirtualKeyCode::Numpad8 | VirtualKeyCode::K => {
                    return Self::try_move_player(0, -1, &gs.ecs)
                }
                VirtualKeyCode::Down | VirtualKeyCode::Numpad2 | VirtualKeyCode::J => {
                    return Self::try_move_player(0, 1, &gs.ecs)
                }
                // Diagonals
                VirtualKeyCode::Numpad9 | VirtualKeyCode::U => {
                    return Self::try_move_player(1, -1, &gs.ecs)
                }
                VirtualKeyCode::Numpad7 | VirtualKeyCode::Y => {
                    return Self::try_move_player(-1, -1, &gs.ecs)
                }
                VirtualKeyCode::Numpad3 | VirtualKeyCode::N => {
                    return Self::try_move_player(1, 1, &gs.ecs)
                }
                VirtualKeyCode::Numpad1 | VirtualKeyCode::B => {
                    return Self::try_move_player(-1, 1, &gs.ecs)
                }
                VirtualKeyCode::G => Self::get_item(&gs.ecs),
                VirtualKeyCode::I => return ShowInventory,
                VirtualKeyCode::D => return ShowDropItem,
                VirtualKeyCode::Escape => return SaveGame,
                VirtualKeyCode::Backslash => return ShowCheatMenu,
                VirtualKeyCode::Period => {
                    if Self::try_next_level(&gs.ecs) {
                        return NextLevel;
                    }
                }
                VirtualKeyCode::Comma => {
                    if Self::try_previous_level(&gs.ecs) {
                        return PreviousLevel;
                    }
                }
                VirtualKeyCode::Numpad5 | VirtualKeyCode::Space => return Self::skip_turn(&gs.ecs),
                VirtualKeyCode::R => return ShowRemoveItem,
                VirtualKeyCode::Q => ctx.quit(),
                _ => return RunState::AwaitingInput,
            },
        }

        RunState::Ticking
    }

    fn get_item(ecs: &World) {
        let player_pos = ecs.fetch::<Point>();
        let player_entity = ecs.fetch::<Entity>();
        let entities = ecs.entities();
        let items = ecs.read_storage::<Item>();
        let positions = ecs.read_storage::<Position>();
        let mut gamelog = ecs.fetch_mut::<GameLog>();

        let mut target_item: Option<Entity> = None;
        for (item_entity, _item, position) in (&entities, &items, &positions).join() {
            if position.x == player_pos.x && position.y == player_pos.y {
                target_item = Some(item_entity);
            }
        }

        match target_item {
            None => gamelog
                .entries
                .push("There is nothing here to pick up.".to_string()),
            Some(item) => {
                let mut pickup = ecs.write_storage::<WantsToPickupItem>();
                pickup
                    .insert(
                        *player_entity,
                        WantsToPickupItem {
                            collected_by: *player_entity,
                            item,
                        },
                    )
                    .expect("Unable to insert want to pickup");
            }
        }
    }

    fn try_next_level(ecs: &World) -> bool {
        let player_pos = ecs.fetch::<Point>();
        let map = ecs.fetch::<Map>();
        let player_idx = map.xy_idx(player_pos.x, player_pos.y);
        return if map.tiles[player_idx] == TileType::DownStairs {
            true
        } else {
            let mut gamelog = ecs.fetch_mut::<GameLog>();
            gamelog
                .entries
                .push("There is no way down from here.".to_string());
            false
        };
    }

    fn try_previous_level(ecs: &World) -> bool {
        let player_pos = ecs.fetch::<Point>();
        let map = ecs.fetch::<Map>();
        let player_idx = map.xy_idx(player_pos.x, player_pos.y);
        return if map.tiles[player_idx] == TileType::UpStairs {
            true
        } else {
            let mut gamelog = ecs.fetch_mut::<GameLog>();
            gamelog
                .entries
                .push("There is no way up from here.".to_string());
            false
        };
    }

    fn skip_turn(ecs: &World) -> RunState {
        let player_entity = ecs.fetch::<Entity>();
        let viewshed_components = ecs.read_storage::<Viewshed>();
        let factions = ecs.read_storage::<Faction>();

        let worldmap_resource = ecs.fetch::<Map>();

        let mut can_heal = true;
        let viewshed = viewshed_components.get(*player_entity).unwrap();
        for tile in &viewshed.visible_tiles {
            let idx = worldmap_resource.xy_idx(tile.x, tile.y);
            spatial::for_each_tile_content(idx, |entity_id| {
                let faction = factions.get(entity_id);
                if let Some(faction) = faction {
                    let reaction = faction_reaction(&faction.name, "Player", &RAWS.lock().unwrap());
                    if reaction == Reaction::Attack {
                        can_heal = false;
                    }
                }
            });
        }

        let hunger_clock = ecs.read_storage::<HungerClock>();
        let hc = hunger_clock.get(*player_entity);
        if let Some(hc) = hc {
            match hc.state {
                HungerState::Hungry | HungerState::Starving => can_heal = false,
                _ => {}
            }
        }
        if can_heal {
            let mut health_components = ecs.write_storage::<Pools>();
            let pools = health_components.get_mut(*player_entity).unwrap();
            pools.hit_points.current = i32::min(pools.hit_points.current + 1, pools.hit_points.max);
            let mut rng = ecs.fetch_mut::<RandomNumberGenerator>();
            if rng.roll_dice(1, 6) == 1 {
                pools.mana.current = i32::min(pools.mana.current + 1, pools.mana.max);
            }
        }

        Ticking
    }
}

fn use_spell_hotkey(gs: &State, key: i32) -> RunState {
    let player_entity = gs.ecs.fetch::<Entity>();
    let known_spells_storage = gs.ecs.read_storage::<KnownSpells>();
    let known_spells = &known_spells_storage.get(*player_entity).unwrap().spells;

    if (key as usize) < known_spells.len() {
        let pools = gs.ecs.read_storage::<Pools>();
        let player_pools = pools.get(*player_entity).unwrap();
        if player_pools.mana.current >= known_spells[key as usize].mana_cost {
            if let Some(spell_entity) =
                find_spell_entity(&gs.ecs, &known_spells[key as usize].display_name)
            {
                if let Some(range) = gs.ecs.read_storage::<Ranged>().get(spell_entity) {
                    return RunState::ShowTargeting {
                        range: range.range,
                        item: spell_entity,
                    };
                }
                let mut intent = gs.ecs.write_storage::<WantsToCastSpell>();
                intent
                    .insert(
                        *player_entity,
                        WantsToCastSpell {
                            spell: spell_entity,
                            target: None,
                        },
                    )
                    .expect("Unable to insert intent");
                return RunState::Ticking;
            }
        } else {
            let mut gamelog = gs.ecs.fetch_mut::<GameLog>();
            gamelog
                .entries
                .push("You don't have enough mana to cast that!".into());
        }
    }

    RunState::Ticking
}

fn use_consumable_hotkey(gs: &State, key: i32) -> RunState {
    let consumables = gs.ecs.read_storage::<Consumable>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let player_entity = gs.ecs.fetch::<Entity>();
    let entities = gs.ecs.entities();
    let mut carried_consumables = Vec::new();
    for (entity, carried_by, _consumable) in (&entities, &backpack, &consumables).join() {
        if carried_by.owner == *player_entity {
            carried_consumables.push(entity);
        }
    }

    if (key as usize) < carried_consumables.len() {
        if let Some(ranged) = gs
            .ecs
            .read_storage::<Ranged>()
            .get(carried_consumables[key as usize])
        {
            return RunState::ShowTargeting {
                range: ranged.range,
                item: carried_consumables[key as usize],
            };
        }
        let mut intent = gs.ecs.write_storage::<WantsToUseItem>();
        intent
            .insert(
                *player_entity,
                WantsToUseItem {
                    item: carried_consumables[key as usize],
                    target: None,
                },
            )
            .expect("Unable to insert intent");
        return Ticking;
    }

    Ticking
}
