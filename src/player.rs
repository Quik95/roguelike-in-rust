use super::components::*;
use super::map::{Map, TileType};
use super::State;
use rltk::{VirtualKeyCode, Point};
use specs::prelude::*;

#[derive(PartialEq, Copy, Clone)]
pub enum RunState {
    Paused,
    Running,
}

impl Player {
    pub fn try_move_player(delta_x: i32, delta_y: i32, ecs: &mut World) {
        let mut positions = ecs.write_storage::<Position>();
        let mut players = ecs.write_storage::<Player>();
        let mut viewsheds = ecs.write_storage::<Viewshed>();
        let map = ecs.fetch::<Map>();
        let mut ppos = ecs.write_resource::<Point>();

        for (_player, pos, viewshed) in (&mut players, &mut positions, &mut viewsheds).join() {
            let destination_idx = Map::xy_idx(pos.x + delta_x, pos.y + delta_y);
            if map.tiles[destination_idx] != TileType::Wall {
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
            None => {return RunState::Paused}
            Some(key) => match key {
                VirtualKeyCode::Left | VirtualKeyCode::Numpad4 | VirtualKeyCode::H => {
                    Player::try_move_player(-1, 0, &mut gs.ecs)
                }
                VirtualKeyCode::Right | VirtualKeyCode::Numpad6 | VirtualKeyCode::L => {
                    Player::try_move_player(1, 0, &mut gs.ecs)
                }
                VirtualKeyCode::Up | VirtualKeyCode::Numpad8 | VirtualKeyCode::K => {
                    Player::try_move_player(0, -1, &mut gs.ecs)
                }
                VirtualKeyCode::Down | VirtualKeyCode::Numpad2 | VirtualKeyCode::J => {
                    Player::try_move_player(0, 1, &mut gs.ecs)
                }
                VirtualKeyCode::Q => {
                    ctx.quit();
                }
                _ => { return RunState::Paused}
            },
        }

        RunState::Running
    }
}
