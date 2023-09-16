use crate::gui::{menu_box, menu_option};
use crate::State;
use rltk::{
    to_cp437, BTerm as Rltk, ColorPair, DrawBatch, Point, VirtualKeyCode, BLACK, RGB, YELLOW,
};

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum CheatMenuResult {
    NoResponse,
    Cancel,
    TeleportToExit,
    Heal,
    Reveal,
    GodMode,
}

pub fn show_cheat_mode(_gs: &mut State, ctx: &Rltk) -> CheatMenuResult {
    let mut draw_batch = DrawBatch::new();
    let count = 4;
    let y = 25 - (count / 2);
    menu_box(&mut draw_batch, 15, y, count + 3, "Cheating!");
    draw_batch.print_color(
        Point::new(18, y + count + 1),
        "ESCAPE to cancel",
        ColorPair::new(RGB::named(YELLOW), RGB::named(BLACK)),
    );

    menu_option(
        &mut draw_batch,
        17,
        y,
        to_cp437('T'),
        "Teleport to next level",
    );
    menu_option(&mut draw_batch, 17, y + 1, to_cp437('H'), "Heal all wounds");
    menu_option(&mut draw_batch, 17, y + 2, to_cp437('R'), "Reveal the map");
    menu_option(
        &mut draw_batch,
        17,
        y + 3,
        to_cp437('G'),
        "God Mode (No Death)",
    );

    draw_batch.submit(6000).expect("Draw batch failed");

    ctx.key
        .map_or(CheatMenuResult::NoResponse, |key| match key {
            VirtualKeyCode::T => CheatMenuResult::TeleportToExit,
            VirtualKeyCode::H => CheatMenuResult::Heal,
            VirtualKeyCode::R => CheatMenuResult::Reveal,
            VirtualKeyCode::G => CheatMenuResult::GodMode,
            VirtualKeyCode::Escape => CheatMenuResult::Cancel,
            _ => CheatMenuResult::NoResponse,
        })
}
