use super::components::*;
use super::map::{xy_idx, TileType};
use super::State;
use rltk::VirtualKeyCode;
use specs::prelude::*;

impl Player {
    pub fn try_move_player(delta_x: i32, delta_y: i32, ecs: &mut World) {
        let mut positions = ecs.write_storage::<Position>();
        let mut players = ecs.write_storage::<Player>();
        let map = ecs.fetch::<Vec<TileType>>();

        for (_player, pos) in (&mut players, &mut positions).join() {
            let destination_idx = xy_idx(pos.x + delta_x, pos.y + delta_y);
            if map[destination_idx] != TileType::Wall {
                pos.x = i32::min(79, i32::max(0, pos.x + delta_x));
                pos.y = i32::min(49, i32::max(0, pos.y + delta_y));
            }
        }
    }

    pub fn player_input(gs: &mut State, ctx: &mut rltk::Rltk) {
        match ctx.key {
            None => {}
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
                _ => {}
            },
        }
    }
}
