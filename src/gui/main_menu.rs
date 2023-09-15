use crate::gui::MainMenuResult::{NoSelection, Selected};
use crate::gui::MainMenuSelection::{LoadGame, NewGame, Quit};
use crate::player::RunState;
use crate::player::RunState::MainMenu;
use crate::rex_assets::RexAssets;
use crate::{saveload_system, State};
use rltk::{ColorPair, DrawBatch, Rect, Rltk, VirtualKeyCode, BLACK, RGB, WHEAT};

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum MainMenuSelection {
    NewGame,
    LoadGame,
    Quit,
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum MainMenuResult {
    NoSelection { selected: MainMenuSelection },
    Selected { selected: MainMenuSelection },
}

pub fn main_menu(gs: &State, ctx: &mut Rltk) -> MainMenuResult {
    let mut draw_batch = DrawBatch::new();

    let save_exists = saveload_system::does_save_exist();
    let runstate = gs.ecs.fetch::<RunState>();
    let assets = gs.ecs.fetch::<RexAssets>();
    ctx.render_xp_sprite(&assets.menu, 0, 0);

    draw_batch.draw_double_box(
        Rect::with_size(24, 18, 31, 10),
        ColorPair::new(RGB::named(WHEAT), RGB::named(BLACK)),
    );

    draw_batch.print_color_centered(
        20,
        "Rust Roguelike Tutorial",
        ColorPair::new(RGB::named(rltk::YELLOW), RGB::named(rltk::BLACK)),
    );
    draw_batch.print_color_centered(
        21,
        "by Herbert Wolverson",
        ColorPair::new(RGB::named(rltk::CYAN), RGB::named(rltk::BLACK)),
    );
    draw_batch.print_color_centered(
        22,
        "Use Up/Down Arrows and Enter",
        ColorPair::new(RGB::named(rltk::GRAY), RGB::named(rltk::BLACK)),
    );

    let mut y = 24;
    if let MainMenu {
        menu_selection: selection,
    } = *runstate
    {
        if selection == MainMenuSelection::NewGame {
            draw_batch.print_color_centered(
                y,
                "Begin New Game",
                ColorPair::new(RGB::named(rltk::MAGENTA), RGB::named(rltk::BLACK)),
            );
        } else {
            draw_batch.print_color_centered(
                y,
                "Begin New Game",
                ColorPair::new(RGB::named(rltk::WHITE), RGB::named(rltk::BLACK)),
            );
        }
        y += 1;

        if save_exists {
            if selection == MainMenuSelection::LoadGame {
                draw_batch.print_color_centered(
                    y,
                    "Load Game",
                    ColorPair::new(RGB::named(rltk::MAGENTA), RGB::named(rltk::BLACK)),
                );
            } else {
                draw_batch.print_color_centered(
                    y,
                    "Load Game",
                    ColorPair::new(RGB::named(rltk::WHITE), RGB::named(rltk::BLACK)),
                );
            }
            y += 1;
        }

        if selection == MainMenuSelection::Quit {
            draw_batch.print_color_centered(
                y,
                "Quit",
                ColorPair::new(RGB::named(rltk::MAGENTA), RGB::named(rltk::BLACK)),
            );
        } else {
            draw_batch.print_color_centered(
                y,
                "Quit",
                ColorPair::new(RGB::named(rltk::WHITE), RGB::named(rltk::BLACK)),
            );
        }

        draw_batch.submit(6000).expect("Batched draw failed");

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
