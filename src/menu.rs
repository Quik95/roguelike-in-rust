use rltk::{BLACK, MAGENTA, RGB, Rltk, VirtualKeyCode, WHITE, YELLOW};

use crate::gui::MainMenuResult;
use crate::gui::MainMenuResult::*;
use crate::gui::MainMenuSelection::*;
use crate::player::RunState::*;
use crate::player::RunState;
use crate::State;

pub fn main_menu(gs: &mut State, ctx: &mut Rltk) -> MainMenuResult {
    let save_exists = super::saveload_system::does_save_exist();
    let runstate = gs.ecs.fetch::<RunState>();

    ctx.print_color_centered(15, RGB::named(YELLOW), RGB::named(BLACK), "Rust Roguelike Tutorial");

    if let MainMenu { menu_selection: selection } = *runstate {
        if selection == NewGame {
            ctx.print_color_centered(24, RGB::named(MAGENTA), RGB::named(BLACK), "Begin New Game");
        } else {
            ctx.print_color_centered(24, RGB::named(WHITE), RGB::named(BLACK), "Begin New Game");
        }

        if save_exists {
            if selection == LoadGame {
                ctx.print_color_centered(25, RGB::named(MAGENTA), RGB::named(BLACK), "Load Game");
            } else {
                ctx.print_color_centered(25, RGB::named(WHITE), RGB::named(BLACK), "Load Game");
            }
        }

        if selection == Quit {
            ctx.print_color_centered(26, RGB::named(MAGENTA), RGB::named(BLACK), "Quit");
        } else {
            ctx.print_color_centered(26, RGB::named(WHITE), RGB::named(BLACK), "Quit");
        }

        match ctx.key {
            None => return NoSelection {selected: selection},
            Some(key) => {
                match key {
                    VirtualKeyCode::Escape => return NoSelection {selected: selection},
                    VirtualKeyCode::Up => {
                        let mut newselection;
                        match selection {
                            NewGame => newselection = Quit,
                            LoadGame => newselection = NewGame,
                            Quit => newselection = LoadGame,
                        }
                        if newselection == LoadGame && !save_exists {
                            newselection = NewGame;
                        }
                        return NoSelection {selected: newselection}
                    }
                    VirtualKeyCode::Down => {
                        let mut newselection;
                        match selection {
                            NewGame => newselection = LoadGame,
                            LoadGame => newselection = Quit,
                            Quit => newselection = NewGame,
                        }
                        if newselection == LoadGame && !save_exists {
                            newselection = Quit;
                        }
                        return NoSelection {selected: newselection}
                    }
                    VirtualKeyCode::Return => return Selected {selected: selection},
                    _ => return NoSelection {selected: selection},
                }
            }
        }
    }


    NoSelection { selected: NewGame }
}