use rltk::{Point, VirtualKeyCode};
use specs::prelude::*;

use crate::gamelog::GameLog;
use crate::gui;
use crate::map::TileType;
use crate::player::RunState::{NextLevel, PlayerTurn, SaveGame, ShowDropItem, ShowInventory, ShowRemoveItem};

use super::components::*;
use super::map::Map;
use super::State;

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum RunState {
    AwaitingInput,
    PreRun,
    PlayerTurn,
    MonsterTurn,
    ShowInventory,
    ShowDropItem,
    ShowTargeting { range: i32, item: Entity },
    MainMenu { menu_selection: gui::MainMenuSelection },
    SaveGame,
    NextLevel,
    ShowRemoveItem,
    GameOver,
    MagicMapReveal { row: i32 },
}

impl Player {
    pub fn try_move_player(delta_x: i32, delta_y: i32, ecs: &mut World) {
        let mut positions = ecs.write_storage::<Position>();
        let mut players = ecs.write_storage::<Self>();
        let mut viewsheds = ecs.write_storage::<Viewshed>();
        let map = ecs.fetch::<Map>();
        let mut ppos = ecs.write_resource::<Point>();
        let combat_stats = ecs.read_storage::<CombatStats>();
        let entities = ecs.entities();
        let mut wants_to_melee = ecs.write_storage::<WantsToMelee>();

        for (entity, _player, pos, viewshed) in (&entities, &mut players, &mut positions, &mut viewsheds).join() {
            if pos.x + delta_x < 1 || pos.x + delta_x > map.width - 1 || pos.y + delta_y < 1 || pos.y + delta_y > map.height - 1 {
                return;
            }
            let destination_idx = Map::xy_idx(pos.x + delta_x, pos.y + delta_y);

            for potential_target in map.tile_content[destination_idx].iter() {
                let target = combat_stats.get(*potential_target);
                if let Some(_target) = target {
                    wants_to_melee.insert(entity, WantsToMelee { target: *potential_target }).expect("Add target failed");
                    return;
                }
            }

            if !map.blocked[destination_idx] {
                pos.x = i32::min(79, i32::max(0, pos.x + delta_x));
                pos.y = i32::min(49, i32::max(0, pos.y + delta_y));
                viewshed.dirty = true;
                ppos.x = pos.x;
                ppos.y = pos.y;
            }
        }
    }

    pub fn player_input(gs: &mut State, ctx: &mut rltk::Rltk) -> RunState {
        match ctx.key {
            None => return RunState::AwaitingInput,
            Some(key) => match key {
                VirtualKeyCode::Left | VirtualKeyCode::Numpad4 | VirtualKeyCode::H => {
                    Self::try_move_player(-1, 0, &mut gs.ecs)
                }
                VirtualKeyCode::Right | VirtualKeyCode::Numpad6 | VirtualKeyCode::L => {
                    Self::try_move_player(1, 0, &mut gs.ecs)
                }
                VirtualKeyCode::Up | VirtualKeyCode::Numpad8 | VirtualKeyCode::K => {
                    Self::try_move_player(0, -1, &mut gs.ecs)
                }
                VirtualKeyCode::Down | VirtualKeyCode::Numpad2 | VirtualKeyCode::J => {
                    Self::try_move_player(0, 1, &mut gs.ecs)
                }
                // Diagonals
                VirtualKeyCode::Numpad9 | VirtualKeyCode::U => {
                    Self::try_move_player(1, -1, &mut gs.ecs)
                }
                VirtualKeyCode::Numpad7 | VirtualKeyCode::Y => {
                    Self::try_move_player(-1, -1, &mut gs.ecs)
                }
                VirtualKeyCode::Numpad3 | VirtualKeyCode::N => {
                    Self::try_move_player(1, 1, &mut gs.ecs)
                }
                VirtualKeyCode::Numpad1 | VirtualKeyCode::B => {
                    Self::try_move_player(-1, 1, &mut gs.ecs)
                }
                VirtualKeyCode::G => Self::get_item(&mut gs.ecs),
                VirtualKeyCode::I => return ShowInventory,
                VirtualKeyCode::D => return ShowDropItem,
                VirtualKeyCode::Escape => return SaveGame,
                VirtualKeyCode::Period => {
                    if Self::try_next_level(&mut gs.ecs) {
                        return NextLevel;
                    }
                }
                VirtualKeyCode::Numpad5 | VirtualKeyCode::Space => return Self::skip_turn(&mut gs.ecs),
                VirtualKeyCode::R => return ShowRemoveItem,
                VirtualKeyCode::Q => ctx.quit(),
                _ => return RunState::AwaitingInput,
            },
        }

        RunState::PlayerTurn
    }

    fn get_item(ecs: &mut World) {
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
            None => gamelog.entries.push("There is nothing here to pick up.".to_string()),
            Some(item) => {
                let mut pickup = ecs.write_storage::<WantsToPickupItem>();
                pickup.insert(*player_entity, WantsToPickupItem { collected_by: *player_entity, item }).expect("Unable to insert want to pickup");
            }
        }
    }

    fn try_next_level(ecs: &mut World) -> bool {
        let player_pos = ecs.fetch::<Point>();
        let map = ecs.fetch::<Map>();
        let player_idx = Map::xy_idx(player_pos.x, player_pos.y);
        return if map.tiles[player_idx] == TileType::DownStairs {
            true
        } else {
            let mut gamelog = ecs.fetch_mut::<GameLog>();
            gamelog.entries.push("There is no way down from here.".to_string());
            false
        };
    }

    fn skip_turn(ecs: &mut World) -> RunState {
        let player_entity = ecs.fetch::<Entity>();
        let viewshed_components = ecs.read_storage::<Viewshed>();
        let monsters = ecs.read_storage::<Monster>();

        let worldmap_resource = ecs.fetch::<Map>();

        let mut can_heal = true;
        let viewshed = viewshed_components.get(*player_entity).unwrap();
        for tile in viewshed.visible_tiles.iter() {
            let idx = Map::xy_idx(tile.x, tile.y);
            for entity_id in worldmap_resource.tile_content[idx].iter() {
                let mob = monsters.get(*entity_id);
                match mob {
                    None => {}
                    Some(_) => can_heal = false
                }
            }
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
            let mut health_components = ecs.write_storage::<CombatStats>();
            let player_hp = health_components.get_mut(*player_entity).unwrap();
            player_hp.hp = i32::min(player_hp.hp + 1, player_hp.max_hp);
        }

        PlayerTurn
    }
}
