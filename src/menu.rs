use rltk::{Rltk, VirtualKeyCode, BLACK, CYAN, GRAY, MAGENTA, RGB, WHEAT, WHITE, YELLOW};

use crate::gui::MainMenuResult;
use crate::gui::MainMenuResult::{NoSelection, Selected};
use crate::gui::MainMenuSelection::{LoadGame, NewGame, Quit};
use crate::player::RunState;
use crate::player::RunState::MainMenu;
use crate::rex_assets::RexAssets;
use crate::State;

pub fn main_menu(gs: &State, ctx: &mut Rltk) -> MainMenuResult {
    let save_exists = super::saveload_system::does_save_exist();
    let runstate = gs.ecs.fetch::<RunState>();
    let assets = gs.ecs.fetch::<RexAssets>();
    ctx.render_xp_sprite(&assets.menu, 0, 0);

    ctx.draw_box_double(24, 18, 31, 10, RGB::named(WHEAT), RGB::named(BLACK));
    ctx.print_color_centered(
        20,
        RGB::named(YELLOW),
        RGB::named(BLACK),
        "Rust Roguelike Tutorial",
    );
    ctx.print_color_centered(
        21,
        RGB::named(CYAN),
        RGB::named(BLACK),
        "by Herbert Wolverson",
    );
    ctx.print_color_centered(
        22,
        RGB::named(GRAY),
        RGB::named(BLACK),
        "Use Up/Down Arrows and Enter",
    );

    let mut y = 24;
    if let MainMenu {
        menu_selection: selection,
    } = *runstate
    {
        if selection == NewGame {
            ctx.print_color_centered(y, RGB::named(MAGENTA), RGB::named(BLACK), "Begin New Game");
        } else {
            ctx.print_color_centered(y, RGB::named(WHITE), RGB::named(BLACK), "Begin New Game");
        }
        y += 1;

        if save_exists {
            if selection == LoadGame {
                ctx.print_color_centered(y, RGB::named(MAGENTA), RGB::named(BLACK), "Load Game");
            } else {
                ctx.print_color_centered(y, RGB::named(WHITE), RGB::named(BLACK), "Load Game");
            }
        }
        y += 1;

        if selection == Quit {
            ctx.print_color_centered(y, RGB::named(MAGENTA), RGB::named(BLACK), "Quit");
        } else {
            ctx.print_color_centered(y, RGB::named(WHITE), RGB::named(BLACK), "Quit");
        }

        match ctx.key {
            None => {
                return NoSelection {
                    selected: selection,
                }
            }
            Some(key) => match key {
                VirtualKeyCode::Escape => {
                    return NoSelection {
                        selected: selection,
                    }
                }
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
                    return NoSelection {
                        selected: newselection,
                    };
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
                    return NoSelection {
                        selected: newselection,
                    };
                }
                VirtualKeyCode::Return => {
                    return Selected {
                        selected: selection,
                    }
                }
                _ => {
                    return NoSelection {
                        selected: selection,
                    }
                }
            },
        }
    }

    NoSelection { selected: NewGame }
}
