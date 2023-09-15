use crate::components::Viewshed;
use crate::gui::ItemMenuResult;
use crate::map::camera::get_screen_bounds;
use crate::State;
use rltk::{BTerm as Rltk, ColorPair, DrawBatch, Point, BLACK, CYAN, RED, RGB, YELLOW};
use specs::{Entity, WorldExt};

pub fn ranged_target(gs: &State, ctx: &mut Rltk, range: i32) -> (ItemMenuResult, Option<Point>) {
    let mut draw_batch = DrawBatch::new();

    let (min_x, max_x, min_y, max_y) = get_screen_bounds(&gs.ecs, ctx);
    let player_entity = gs.ecs.fetch::<Entity>();
    let player_pos = gs.ecs.fetch::<Point>();
    let viewsheds = gs.ecs.read_storage::<Viewshed>();

    draw_batch.print_color(
        Point::new(5, 0),
        "Select Target:",
        ColorPair::new(RGB::named(YELLOW), RGB::named(BLACK)),
    );

    // Highlight available target cells
    let mut available_cells = Vec::new();
    let visible = viewsheds.get(*player_entity);
    if let Some(visible) = visible {
        // We have a viewshed
        for idx in &visible.visible_tiles {
            let distance = rltk::DistanceAlg::Pythagoras.distance2d(*player_pos, *idx);
            if distance <= range as f32 {
                let screen_x = idx.x - min_x;
                let screen_y = idx.y - min_y;
                if screen_x > 1
                    && screen_x < (max_x - min_x) - 1
                    && screen_y > 1
                    && screen_y < (max_y - min_y) - 1
                {
                    ctx.set_bg(screen_x, screen_y, RGB::named(rltk::BLUE));
                    available_cells.push(idx);
                }
            }
        }
    } else {
        return (ItemMenuResult::Cancel, None);
    }

    // Draw mouse cursor
    let mouse_pos = ctx.mouse_pos();
    let mut mouse_map_pos = mouse_pos;
    mouse_map_pos.0 += min_x;
    mouse_map_pos.1 += min_y;

    let mut valid_target = false;
    for idx in &available_cells {
        if idx.x == mouse_map_pos.0 && idx.y == mouse_map_pos.1 {
            valid_target = true;
        }
    }
    if valid_target {
        draw_batch.set_bg(Point::new(mouse_pos.0, mouse_pos.1), RGB::named(CYAN));
        if ctx.left_click {
            return (
                ItemMenuResult::Selected,
                Some(Point::new(mouse_map_pos.0, mouse_map_pos.1)),
            );
        }
    } else {
        draw_batch.set_bg(Point::new(mouse_pos.0, mouse_pos.1), RGB::named(RED));
        if ctx.left_click {
            return (ItemMenuResult::Cancel, None);
        }
    }

    draw_batch.submit(5000).expect("Batched draw failed");

    (ItemMenuResult::NoResponse, None)
}
