use super::components::*;
use super::map::{Map};
use super::State;
use rltk::{Point, VirtualKeyCode};
use specs::prelude::*;

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum RunState {
    AwaitingInput,
    PreRun,
    PlayerTurn,
    MonsterTurn
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
                VirtualKeyCode::Q => {
                    ctx.quit();
                }
                _ => return RunState::AwaitingInput,
            },
        }

        RunState::PlayerTurn
    }
}
