use lazy_static::lazy_static;
use rltk::{to_cp437, Point, Rltk, VirtualKeyCode, GOLD, GREEN, MAGENTA, RED, RGB, WHITE, YELLOW};
use specs::prelude::*;

use crate::camera::get_screen_bounds;
use crate::components::HungerState::{Normal, WellFed};
use crate::components::{
    Attribute, Attributes, Consumable, CursedItem, Equipped, Hidden, HungerClock, HungerState,
    Item, MagicItem, MagicItemClass, ObfuscatedName, Pools, Vendor,
};
use crate::map::dungeon::MasterDungeonMap;
use crate::player::VendorMode;
use crate::raws::rawmaster::{get_vendor_items, RAWS};

use super::{gamelog::GameLog, InBackpack, Map, Name, Position, State, Viewshed};

lazy_static! {
    static ref BLACK: RGB = RGB::named(rltk::BLACK);
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum VendorResult {
    NoResponse,
    Cancel,
    Sell,
    BuyMode,
    SellMode,
    Buy,
}

pub fn draw_ui(ecs: &World, ctx: &mut Rltk) {
    let box_gray = RGB::from_hex("#999999").unwrap();
    let black = *BLACK;
    let white = RGB::named(WHITE);

    draw_hollow_box(ctx, 0, 0, 79, 59, box_gray, black);
    draw_hollow_box(ctx, 0, 0, 49, 45, box_gray, black);
    draw_hollow_box(ctx, 0, 45, 79, 14, box_gray, black);
    draw_hollow_box(ctx, 49, 0, 30, 8, box_gray, black);

    ctx.set(0, 45, box_gray, black, to_cp437('├'));
    ctx.set(49, 8, box_gray, black, to_cp437('├'));
    ctx.set(49, 0, box_gray, black, to_cp437('┬'));
    ctx.set(49, 45, box_gray, black, to_cp437('┴'));
    ctx.set(79, 8, box_gray, black, to_cp437('┤'));
    ctx.set(79, 45, box_gray, black, to_cp437('┤'));

    let map = ecs.fetch::<Map>();
    let name_length = map.name.len() + 2;
    let x_pos = (22 - (name_length / 2)) as i32;
    ctx.set(x_pos, 0, box_gray, black, to_cp437('┤'));
    ctx.set(
        x_pos + name_length as i32,
        0,
        box_gray,
        black,
        to_cp437('├'),
    );
    ctx.print_color(x_pos + 1, 0, white, black, &map.name);
    std::mem::drop(map);

    let player_entity = ecs.fetch::<Entity>();
    let pools = ecs.read_storage::<Pools>();
    let player_pools = pools.get(*player_entity).unwrap();
    let health = format!(
        "Health: {}/{}",
        player_pools.hit_points.current, player_pools.hit_points.max
    );
    let mana = format!(
        "Mana:   {}/{}",
        player_pools.mana.current, player_pools.mana.max
    );
    ctx.print_color(50, 1, white, black, &health);
    ctx.print_color(50, 2, white, black, &mana);
    ctx.draw_bar_horizontal(
        64,
        1,
        14,
        player_pools.hit_points.current,
        player_pools.hit_points.max,
        RGB::named(rltk::RED),
        *BLACK,
    );
    ctx.draw_bar_horizontal(
        64,
        2,
        14,
        player_pools.mana.current,
        player_pools.mana.max,
        RGB::named(rltk::BLUE),
        *BLACK,
    );

    let xp = format!("Level:  {}", player_pools.level);
    ctx.print_color(50, 3, white, black, &xp);
    let xp_level_start = (player_pools.level - 1) * 1000;
    ctx.draw_bar_horizontal(
        64,
        3,
        14,
        player_pools.xp - xp_level_start,
        1000,
        RGB::named(GOLD),
        *BLACK,
    );

    let attributes = ecs.read_storage::<Attributes>();
    let attr = attributes.get(*player_entity).unwrap();
    draw_attribute("Might:", &attr.might, 4, ctx);
    draw_attribute("Quickness:", &attr.quickness, 5, ctx);
    draw_attribute("Fitness:", &attr.fitness, 6, ctx);
    draw_attribute("Intelligence:", &attr.intelligence, 7, ctx);

    ctx.print_color(
        50,
        9,
        white,
        black,
        &format!(
            "{:.0} kg ({} kg max)",
            player_pools.total_weight,
            attr.get_max_carry_capacity()
        ),
    );
    ctx.print_color(
        50,
        10,
        white,
        black,
        &format!(
            "Initiative Penalty: {:.0}",
            player_pools.total_initiative_penalty
        ),
    );
    ctx.print_color(
        50,
        11,
        RGB::named(GOLD),
        black,
        &format!("Gold: {:.1}", player_pools.gold),
    );

    let mut y = 13;
    let entities = ecs.entities();
    let equipped = ecs.read_storage::<Equipped>();
    for (entity, equipped_by) in (&entities, &equipped).join() {
        if equipped_by.owner == *player_entity {
            ctx.print_color(
                50,
                y,
                get_item_color(ecs, entity),
                black,
                &get_item_display_name(ecs, entity),
            );
            y += 1;
        }
    }

    y += 1;
    let _green = RGB::from_f32(0.0, 1.0, 0.0);
    let yellow = RGB::named(rltk::YELLOW);
    let consumables = ecs.read_storage::<Consumable>();
    let backpack = ecs.read_storage::<InBackpack>();
    let mut index = 1;
    for (entity, carried_by, _consumable) in (&entities, &backpack, &consumables).join() {
        if carried_by.owner == *player_entity && index < 10 {
            ctx.print_color(50, y, yellow, black, &format!("↑{index}"));
            ctx.print_color(
                53,
                y,
                get_item_color(ecs, entity),
                black,
                &get_item_display_name(ecs, entity),
            );
            y += 1;
            index += 1;
        }
    }

    let hunger = ecs.read_storage::<HungerClock>();
    let hc = hunger.get(*player_entity).unwrap();
    match hc.state {
        WellFed => ctx.print_color(50, 44, RGB::named(GREEN), *BLACK, "Well Fed"),
        Normal => {}
        HungerState::Hungry => ctx.print_color(50, 44, RGB::named(rltk::ORANGE), *BLACK, "Hungry"),
        HungerState::Starving => ctx.print_color(50, 44, RGB::named(rltk::RED), *BLACK, "Starving"),
    }

    let log = ecs.fetch::<GameLog>();
    let mut y = 46;
    for s in log.entries.iter().rev() {
        if y < 59 {
            ctx.print(2, y, s);
        }
        y += 1;
    }

    draw_tooltips(ecs, ctx);
}

fn draw_attribute(name: &str, attribute: &Attribute, y: i32, ctx: &mut Rltk) {
    let black = *BLACK;
    let attr_gray: RGB = RGB::from_hex("#CCCCCC").expect("Oops");
    ctx.print_color(50, y, attr_gray, black, name);
    let color: RGB = match attribute.modifiers {
        0 => RGB::named(rltk::WHITE),
        1.. => RGB::from_f32(0.0, 1.0, 0.0),
        _ => RGB::from_f32(1.0, 0.0, 0.0),
    };
    ctx.print_color(
        67,
        y,
        color,
        black,
        &format!("{}", attribute.base + attribute.modifiers),
    );
    ctx.print_color(73, y, color, black, &format!("{}", attribute.bonus));
    if attribute.bonus > 0 {
        ctx.set(72, y, color, black, rltk::to_cp437('+'));
    }
}

pub fn draw_hollow_box(
    console: &mut Rltk,
    sx: i32,
    sy: i32,
    width: i32,
    height: i32,
    fg: RGB,
    bg: RGB,
) {
    console.set(sx, sy, fg, bg, to_cp437('┌'));
    console.set(sx + width, sy, fg, bg, to_cp437('┐'));
    console.set(sx, sy + height, fg, bg, to_cp437('└'));
    console.set(sx + width, sy + height, fg, bg, to_cp437('┘'));
    for x in sx + 1..sx + width {
        console.set(x, sy, fg, bg, to_cp437('─'));
        console.set(x, sy + height, fg, bg, to_cp437('─'));
    }
    for y in sy + 1..sy + height {
        console.set(sx, y, fg, bg, to_cp437('│'));
        console.set(sx + width, y, fg, bg, to_cp437('│'));
    }
}

fn draw_tooltips(ecs: &World, ctx: &mut Rltk) {
    let (min_x, _max_x, min_y, _max_y) = get_screen_bounds(ecs, ctx);
    let map = ecs.fetch::<Map>();
    let positions = ecs.read_storage::<Position>();
    let hidden = ecs.read_storage::<Hidden>();
    let attributes = ecs.read_storage::<Attributes>();
    let pools = ecs.read_storage::<Pools>();
    let entities = ecs.entities();

    let mouse_pos = ctx.mouse_pos();
    let mut mouse_map_pos = mouse_pos;
    mouse_map_pos.0 += min_x;
    mouse_map_pos.1 += min_y;
    if mouse_map_pos.0 >= map.width - 1
        || mouse_map_pos.1 >= map.height - 1
        || mouse_map_pos.0 < 1
        || mouse_map_pos.1 < 1
    {
        return;
    }
    if !map.visible_tiles[map.xy_idx(mouse_map_pos.0, mouse_map_pos.1)] {
        return;
    }

    let mut tip_boxes: Vec<Tooltip> = Vec::new();
    for (entity, position, _hidden) in (&entities, &positions, !&hidden).join() {
        if position.x == mouse_map_pos.0 && position.y == mouse_map_pos.1 {
            let mut tip = Tooltip::default();
            tip.add(get_item_display_name(ecs, entity));

            // Comment on attributes
            let attr = attributes.get(entity);
            if let Some(attr) = attr {
                let mut s = String::new();
                if attr.might.bonus < 0 {
                    s += "Weak. ";
                };
                if attr.might.bonus > 0 {
                    s += "Strong. ";
                };
                if attr.quickness.bonus < 0 {
                    s += "Clumsy. ";
                };
                if attr.quickness.bonus > 0 {
                    s += "Agile. ";
                };
                if attr.fitness.bonus < 0 {
                    s += "Unheathy. ";
                };
                if attr.fitness.bonus > 0 {
                    s += "Healthy.";
                };
                if attr.intelligence.bonus < 0 {
                    s += "Unintelligent. ";
                };
                if attr.intelligence.bonus > 0 {
                    s += "Smart. ";
                };
                if s.is_empty() {
                    s = "Quite Average".to_string();
                }
                tip.add(s);
            }

            // Comment on pools
            let stat = pools.get(entity);
            if let Some(stat) = stat {
                tip.add(format!("Level: {}", stat.level));
            }

            tip_boxes.push(tip);
        }
    }

    if tip_boxes.is_empty() {
        return;
    }

    let box_gray: RGB = RGB::from_hex("#999999").expect("Oops");
    let white = RGB::named(rltk::WHITE);

    let arrow;
    let arrow_x;
    let arrow_y = mouse_pos.1;
    if mouse_pos.0 < 40 {
        // Render to the left
        arrow = to_cp437('→');
        arrow_x = mouse_pos.0 - 1;
    } else {
        // Render to the right
        arrow = to_cp437('←');
        arrow_x = mouse_pos.0 + 1;
    }
    ctx.set(arrow_x, arrow_y, white, box_gray, arrow);

    let mut total_height = 0;
    for tt in &tip_boxes {
        total_height += tt.height();
    }

    let mut y = mouse_pos.1 - (total_height / 2);
    while y + (total_height / 2) > 50 {
        y -= 1;
    }

    for tt in &tip_boxes {
        let x = if mouse_pos.0 < 40 {
            mouse_pos.0 - (1 + tt.width())
        } else {
            mouse_pos.0 + (1 + tt.width())
        };
        tt.render(ctx, x, y);
        y += tt.height();
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum ItemMenuResult {
    Cancel,
    NoResponse,
    Selected,
}

pub fn show_inventory(gs: &State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let entities = gs.ecs.entities();

    let inventory = (&backpack, &names)
        .join()
        .filter(|item| item.0.owner == *player_entity);
    let count = inventory.count();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(
        15,
        y - 2,
        31,
        (count + 3) as i32,
        RGB::named(rltk::WHITE),
        *BLACK,
    );
    ctx.print_color(18, y - 2, RGB::named(rltk::YELLOW), *BLACK, "Inventory");
    ctx.print_color(
        18,
        y + count as i32 + 1,
        RGB::named(rltk::YELLOW),
        *BLACK,
        "ESCAPE to cancel",
    );

    let mut equippable: Vec<Entity> = Vec::new();
    for (j, (entity, _pack)) in (&entities, &backpack)
        .join()
        .filter(|item| item.1.owner == *player_entity)
        .enumerate()
    {
        ctx.set(17, y, RGB::named(rltk::WHITE), *BLACK, rltk::to_cp437('('));
        ctx.set(
            18,
            y,
            RGB::named(rltk::YELLOW),
            *BLACK,
            97 + j as rltk::FontCharType,
        );
        ctx.set(19, y, RGB::named(rltk::WHITE), *BLACK, rltk::to_cp437(')'));

        ctx.print_color(
            21,
            y,
            get_item_color(&gs.ecs, entity),
            *BLACK,
            &get_item_display_name(&gs.ecs, entity),
        );
        equippable.push(entity);
        y += 1;
    }

    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(key) => match key {
            VirtualKeyCode::Escape => (ItemMenuResult::Cancel, None),
            _ => {
                let selection = rltk::letter_to_option(key);
                if selection > -1 && selection < count as i32 {
                    return (
                        ItemMenuResult::Selected,
                        Some(equippable[selection as usize]),
                    );
                }
                (ItemMenuResult::NoResponse, None)
            }
        },
    }
}

pub fn drop_item_menu(gs: &State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let entities = gs.ecs.entities();

    let inventory = (&backpack, &names)
        .join()
        .filter(|item| item.0.owner == *player_entity);
    let count = inventory.count();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(
        15,
        y - 2,
        31,
        (count + 3) as i32,
        RGB::named(rltk::WHITE),
        *BLACK,
    );
    ctx.print_color(
        18,
        y - 2,
        RGB::named(rltk::YELLOW),
        *BLACK,
        "Drop Which Item?",
    );
    ctx.print_color(
        18,
        y + count as i32 + 1,
        RGB::named(rltk::YELLOW),
        *BLACK,
        "ESCAPE to cancel",
    );

    let mut equippable: Vec<Entity> = Vec::new();
    for (j, (entity, _pack)) in (&entities, &backpack)
        .join()
        .filter(|item| item.1.owner == *player_entity)
        .enumerate()
    {
        ctx.set(17, y, RGB::named(rltk::WHITE), *BLACK, rltk::to_cp437('('));
        ctx.set(
            18,
            y,
            RGB::named(rltk::YELLOW),
            *BLACK,
            97 + j as rltk::FontCharType,
        );
        ctx.set(19, y, RGB::named(rltk::WHITE), *BLACK, rltk::to_cp437(')'));

        ctx.print_color(
            21,
            y,
            get_item_color(&gs.ecs, entity),
            *BLACK,
            &get_item_display_name(&gs.ecs, entity),
        );
        equippable.push(entity);
        y += 1;
    }

    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(key) => match key {
            VirtualKeyCode::Escape => (ItemMenuResult::Cancel, None),
            _ => {
                let selection = rltk::letter_to_option(key);
                if selection > -1 && selection < count as i32 {
                    return (
                        ItemMenuResult::Selected,
                        Some(equippable[selection as usize]),
                    );
                }
                (ItemMenuResult::NoResponse, None)
            }
        },
    }
}

pub fn ranged_target(gs: &State, ctx: &mut Rltk, range: i32) -> (ItemMenuResult, Option<Point>) {
    let (min_x, max_x, min_y, max_y) = get_screen_bounds(&gs.ecs, ctx);
    let player_entity = gs.ecs.fetch::<Entity>();
    let player_pos = gs.ecs.fetch::<Point>();
    let viewsheds = gs.ecs.read_storage::<Viewshed>();

    ctx.print_color(5, 0, RGB::named(rltk::YELLOW), *BLACK, "Select Target:");

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
                    ctx.set_bg(idx.x, idx.y, RGB::named(rltk::BLUE));
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
        ctx.set_bg(mouse_pos.0, mouse_pos.1, RGB::named(rltk::CYAN));
        if ctx.left_click {
            return (
                ItemMenuResult::Selected,
                Some(Point::new(mouse_map_pos.0, mouse_map_pos.1)),
            );
        }
    } else {
        ctx.set_bg(mouse_pos.0, mouse_pos.1, RGB::named(rltk::RED));
        if ctx.left_click {
            return (ItemMenuResult::Cancel, None);
        }
    }

    (ItemMenuResult::NoResponse, None)
}

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

pub fn remove_item_menu(gs: &State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let backpack = gs.ecs.read_storage::<Equipped>();
    let entities = gs.ecs.entities();

    let inventory = (&backpack, &names)
        .join()
        .filter(|item| item.0.owner == *player_entity);
    let count = inventory.count();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(15, y - 2, 31, (count + 3) as i32, RGB::named(WHITE), *BLACK);
    ctx.print_color(18, y - 2, RGB::named(YELLOW), *BLACK, "Remove Which Item?");
    ctx.print_color(
        18,
        y + count as i32 + 1,
        RGB::named(YELLOW),
        *BLACK,
        "ESCAPE to cancel",
    );

    let mut equippable: Vec<Entity> = Vec::new();
    for (j, (entity, _pack)) in (&entities, &backpack)
        .join()
        .filter(|item| item.1.owner == *player_entity)
        .enumerate()
    {
        ctx.set(17, y, RGB::named(WHITE), *BLACK, rltk::to_cp437('('));
        ctx.set(
            18,
            y,
            RGB::named(YELLOW),
            *BLACK,
            97 + j as rltk::FontCharType,
        );
        ctx.set(19, y, RGB::named(WHITE), *BLACK, rltk::to_cp437(')'));

        ctx.print_color(
            21,
            y,
            get_item_color(&gs.ecs, entity),
            *BLACK,
            &get_item_display_name(&gs.ecs, entity),
        );
        equippable.push(entity);
        y += 1;
    }

    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(key) => match key {
            VirtualKeyCode::Escape => (ItemMenuResult::Cancel, None),
            _ => {
                let selection = rltk::letter_to_option(key);
                if selection > -1 && selection < count as i32 {
                    return (
                        ItemMenuResult::Selected,
                        Some(equippable[selection as usize]),
                    );
                }
                (ItemMenuResult::NoResponse, None)
            }
        },
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum GameOverResult {
    NoSelection,
    QuitToMenu,
}

pub fn game_over(ctx: &mut Rltk) -> GameOverResult {
    ctx.print_color_centered(15, RGB::named(YELLOW), *BLACK, "Your journey has ended!");
    ctx.print_color_centered(
        17,
        RGB::named(WHITE),
        *BLACK,
        "One day, we'll tell you all about how you did.",
    );
    ctx.print_color_centered(
        18,
        RGB::named(WHITE),
        *BLACK,
        "That day, sadly, is not in this chapter...",
    );

    ctx.print_color_centered(
        20,
        RGB::named(MAGENTA),
        *BLACK,
        "Press ENTER to return to the menu.",
    );

    match ctx.key {
        None => GameOverResult::NoSelection,
        Some(_) => GameOverResult::QuitToMenu,
    }
}

#[derive(Default)]
struct Tooltip {
    lines: Vec<String>,
}

impl Tooltip {
    fn add(&mut self, line: impl ToString) {
        self.lines.push(line.to_string());
    }

    fn width(&self) -> i32 {
        let mut max = 0;
        for s in &self.lines {
            if s.len() > max {
                max = s.len();
            }
        }
        max as i32 + 2
    }

    fn height(&self) -> i32 {
        self.lines.len() as i32 + 2
    }

    fn render(&self, ctx: &mut Rltk, x: i32, y: i32) {
        let box_gray: RGB = RGB::from_hex("#999999").expect("Oops");
        let light_gray: RGB = RGB::from_hex("#DDDDDD").expect("Oops");
        let white = RGB::named(rltk::WHITE);
        let black = *BLACK;
        ctx.draw_box(x, y, self.width() - 1, self.height() - 1, white, box_gray);
        for (i, s) in self.lines.iter().enumerate() {
            let col = if i == 0 { white } else { light_gray };
            ctx.print_color(x + 1, y + i as i32 + 1, col, black, s);
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum CheatMenuResult {
    NoResponse,
    Cancel,
    TeleportToExit,
    Heal,
    Reveal,
    GodMode,
}

pub fn show_cheat_mode(_gs: &mut State, ctx: &mut Rltk) -> CheatMenuResult {
    let count = 4;
    let mut y = 25 - (count / 2);
    ctx.draw_box(15, y - 2, 31, count + 3, RGB::named(WHITE), *BLACK);
    ctx.print_color(18, y - 2, RGB::named(YELLOW), *BLACK, "Cheating!");
    ctx.print_color(
        18,
        y + count + 1,
        RGB::named(YELLOW),
        *BLACK,
        "ESCAPE to cancel",
    );

    ctx.set(17, y, RGB::named(WHITE), *BLACK, to_cp437('('));
    ctx.set(18, y, RGB::named(YELLOW), *BLACK, to_cp437('T'));
    ctx.set(19, y, RGB::named(WHITE), *BLACK, to_cp437(')'));
    ctx.print(21, y, "Teleport to next level");

    y += 1;
    ctx.set(17, y, RGB::named(WHITE), *BLACK, to_cp437('('));
    ctx.set(18, y, RGB::named(YELLOW), *BLACK, to_cp437('H'));
    ctx.set(19, y, RGB::named(WHITE), *BLACK, to_cp437(')'));
    ctx.print(21, y, "Heal all wounds");

    y += 1;
    ctx.set(17, y, RGB::named(WHITE), *BLACK, to_cp437('('));
    ctx.set(18, y, RGB::named(YELLOW), *BLACK, to_cp437('R'));
    ctx.set(19, y, RGB::named(WHITE), *BLACK, to_cp437(')'));
    ctx.print(21, y, "Reveal the map");

    y += 1;
    ctx.set(17, y, RGB::named(WHITE), *BLACK, to_cp437('('));
    ctx.set(18, y, RGB::named(YELLOW), *BLACK, to_cp437('G'));
    ctx.set(19, y, RGB::named(WHITE), *BLACK, to_cp437(')'));
    ctx.print(21, y, "God Mode (No Death)");

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

pub fn show_vendor_menu(
    gs: &State,
    ctx: &mut Rltk,
    vendor: Entity,
    mode: VendorMode,
) -> (VendorResult, Option<Entity>, Option<String>, Option<f32>) {
    match mode {
        VendorMode::Buy => vendor_buy_menu(gs, ctx, vendor, mode),
        VendorMode::Sell => vendor_sell_menu(gs, ctx, vendor, mode),
    }
}

fn vendor_sell_menu(
    gs: &State,
    ctx: &mut Rltk,
    _vendor: Entity,
    _mode: VendorMode,
) -> (VendorResult, Option<Entity>, Option<String>, Option<f32>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let items = gs.ecs.read_storage::<Item>();
    let entities = gs.ecs.entities();

    let inventory = (&backpack, &names)
        .join()
        .filter(|item| item.0.owner == *player_entity);
    let count = inventory.count();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(
        15,
        y - 2,
        51,
        (count + 3) as i32,
        RGB::named(rltk::WHITE),
        *BLACK,
    );
    ctx.print_color(
        18,
        y - 2,
        RGB::named(rltk::YELLOW),
        *BLACK,
        "Sell Which Item? (space to switch to buy mode)",
    );
    ctx.print_color(
        18,
        y + count as i32 + 1,
        RGB::named(rltk::YELLOW),
        *BLACK,
        "ESCAPE to cancel",
    );

    let mut equippable: Vec<Entity> = Vec::new();
    for (j, (entity, _pack, item)) in (&entities, &backpack, &items)
        .join()
        .filter(|item| item.1.owner == *player_entity)
        .enumerate()
    {
        ctx.set(17, y, RGB::named(rltk::WHITE), *BLACK, rltk::to_cp437('('));
        ctx.set(
            18,
            y,
            RGB::named(rltk::YELLOW),
            *BLACK,
            97 + j as rltk::FontCharType,
        );
        ctx.set(19, y, RGB::named(rltk::WHITE), *BLACK, rltk::to_cp437(')'));

        ctx.print_color(
            21,
            y,
            get_item_color(&gs.ecs, entity),
            *BLACK,
            &get_item_display_name(&gs.ecs, entity),
        );
        ctx.print(50, y, &format!("{:.1} gp", item.base_value * 0.8));
        equippable.push(entity);
        y += 1;
    }

    match ctx.key {
        None => (VendorResult::NoResponse, None, None, None),
        Some(key) => match key {
            VirtualKeyCode::Space => (VendorResult::BuyMode, None, None, None),
            VirtualKeyCode::Escape => (VendorResult::Cancel, None, None, None),
            _ => {
                let selection = rltk::letter_to_option(key);
                if selection > -1 && selection < count as i32 {
                    return (
                        VendorResult::Sell,
                        Some(equippable[selection as usize]),
                        None,
                        None,
                    );
                }
                (VendorResult::NoResponse, None, None, None)
            }
        },
    }
}

fn vendor_buy_menu(
    gs: &State,
    ctx: &mut Rltk,
    vendor: Entity,
    _mode: VendorMode,
) -> (VendorResult, Option<Entity>, Option<String>, Option<f32>) {
    let vendors = gs.ecs.read_storage::<Vendor>();

    let inventory = get_vendor_items(
        &vendors.get(vendor).unwrap().categories,
        &RAWS.lock().unwrap(),
    );
    let count = inventory.len();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(
        15,
        y - 2,
        51,
        (count + 3) as i32,
        RGB::named(rltk::WHITE),
        *BLACK,
    );
    ctx.print_color(
        18,
        y - 2,
        RGB::named(rltk::YELLOW),
        *BLACK,
        "Buy Which Item? (space to switch to sell mode)",
    );
    ctx.print_color(
        18,
        y + count as i32 + 1,
        RGB::named(rltk::YELLOW),
        *BLACK,
        "ESCAPE to cancel",
    );

    for (j, sale) in inventory.iter().enumerate() {
        ctx.set(17, y, RGB::named(rltk::WHITE), *BLACK, rltk::to_cp437('('));
        ctx.set(
            18,
            y,
            RGB::named(rltk::YELLOW),
            *BLACK,
            97 + j as rltk::FontCharType,
        );
        ctx.set(19, y, RGB::named(rltk::WHITE), *BLACK, rltk::to_cp437(')'));

        ctx.print(21, y, &sale.0);
        ctx.print(50, y, &format!("{:.1} gp", sale.1 * 1.2));
        y += 1;
    }

    match ctx.key {
        None => (VendorResult::NoResponse, None, None, None),
        Some(key) => match key {
            VirtualKeyCode::Space => (VendorResult::SellMode, None, None, None),
            VirtualKeyCode::Escape => (VendorResult::Cancel, None, None, None),
            _ => {
                let selection = rltk::letter_to_option(key);
                if selection > -1 && selection < count as i32 {
                    return (
                        VendorResult::Buy,
                        None,
                        Some(inventory[selection as usize].0.clone()),
                        Some(inventory[selection as usize].1),
                    );
                }
                (VendorResult::NoResponse, None, None, None)
            }
        },
    }
}

pub fn get_item_color(ecs: &World, item: Entity) -> RGB {
    let dm = ecs.fetch::<MasterDungeonMap>();
    if let Some(name) = ecs.read_storage::<Name>().get(item) {
        if ecs.read_storage::<CursedItem>().get(item).is_some()
            && dm.identified_items.contains(&name.name)
        {
            return RED.into();
        }
    }

    if let Some(magic) = ecs.read_storage::<MagicItem>().get(item) {
        return match magic.class {
            MagicItemClass::Common => RGB::from_f32(0.5, 1.0, 0.5),
            MagicItemClass::Rare => RGB::from_f32(0.0, 1.0, 1.0),
            MagicItemClass::Legendary => RGB::from_f32(0.71, 0.15, 0.93),
        };
    }
    RGB::from_f32(1.0, 1.0, 1.0)
}

pub fn get_item_display_name(ecs: &World, item: Entity) -> String {
    ecs.read_storage::<Name>()
        .get(item)
        .map_or("Nameless item (bug)".into(), |name| {
            if ecs.read_storage::<MagicItem>().get(item).is_some() {
                let dm = ecs.fetch::<MasterDungeonMap>();
                if dm.identified_items.contains(&name.name) {
                    name.name.clone()
                } else if let Some(obfuscated) = ecs.read_storage::<ObfuscatedName>().get(item) {
                    obfuscated.name.clone()
                } else {
                    "Unidentified magic item".to_string()
                }
            } else {
                name.name.clone()
            }
        })
}

pub fn remove_curse_menu(gs: &State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let equipped = gs.ecs.read_storage::<Equipped>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let entities = gs.ecs.entities();
    let items = gs.ecs.read_storage::<Item>();
    let cursed = gs.ecs.read_storage::<CursedItem>();
    let names = gs.ecs.read_storage::<Name>();
    let dm = gs.ecs.fetch::<MasterDungeonMap>();

    let build_cursed_iterator = || {
        (&entities, &items, &cursed)
            .join()
            .filter(|(item_entity, _item, _cursed)| {
                let mut keep = false;
                if let Some(bp) = backpack.get(*item_entity) {
                    if bp.owner == *player_entity {
                        if let Some(name) = names.get(*item_entity) {
                            if dm.identified_items.contains(&name.name) {
                                keep = true;
                            }
                        }
                    }
                }
                // It's equipped, so we know it's cursed
                if let Some(equip) = equipped.get(*item_entity) {
                    if equip.owner == *player_entity {
                        keep = true;
                    }
                }
                keep
            })
    };

    let count = build_cursed_iterator().count();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(15, y - 2, 31, (count + 3) as i32, RGB::named(WHITE), *BLACK);
    ctx.print_color(
        18,
        y - 2,
        RGB::named(YELLOW),
        *BLACK,
        "Remove Curse From Which Item?",
    );
    ctx.print_color(
        18,
        y + count as i32 + 1,
        RGB::named(YELLOW),
        *BLACK,
        "ESCAPE to cancel",
    );

    let mut equippable: Vec<Entity> = Vec::new();
    for (j, (entity, _item, _cursed)) in build_cursed_iterator().enumerate() {
        ctx.set(17, y, RGB::named(WHITE), *BLACK, rltk::to_cp437('('));
        ctx.set(
            18,
            y,
            RGB::named(YELLOW),
            *BLACK,
            97 + j as rltk::FontCharType,
        );
        ctx.set(19, y, RGB::named(WHITE), *BLACK, rltk::to_cp437(')'));

        ctx.print_color(
            21,
            y,
            get_item_color(&gs.ecs, entity),
            *BLACK,
            &get_item_display_name(&gs.ecs, entity),
        );
        equippable.push(entity);
        y += 1;
    }

    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(key) => match key {
            VirtualKeyCode::Escape => (ItemMenuResult::Cancel, None),
            _ => {
                let selection = rltk::letter_to_option(key);
                if selection > -1 && selection < count as i32 {
                    return (
                        ItemMenuResult::Selected,
                        Some(equippable[selection as usize]),
                    );
                }
                (ItemMenuResult::NoResponse, None)
            }
        },
    }
}

pub fn identify_menu(gs: &State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let equipped = gs.ecs.read_storage::<Equipped>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let entities = gs.ecs.entities();
    let items = gs.ecs.read_storage::<Item>();
    let names = gs.ecs.read_storage::<Name>();
    let dm = gs.ecs.fetch::<MasterDungeonMap>();
    let obfuscated = gs.ecs.read_storage::<ObfuscatedName>();

    let build_cursed_iterator = || {
        (&entities, &items).join().filter(|(item_entity, _item)| {
            let mut keep = false;
            if let Some(bp) = backpack.get(*item_entity) {
                if bp.owner == *player_entity {
                    if let Some(name) = names.get(*item_entity) {
                        if obfuscated.get(*item_entity).is_some()
                            && !dm.identified_items.contains(&name.name)
                        {
                            keep = true;
                        }
                    }
                }
            }
            // It's equipped, so we know it's cursed
            if let Some(equip) = equipped.get(*item_entity) {
                if equip.owner == *player_entity {
                    if let Some(name) = names.get(*item_entity) {
                        if obfuscated.get(*item_entity).is_some()
                            && !dm.identified_items.contains(&name.name)
                        {
                            keep = true;
                        }
                    }
                }
            }
            keep
        })
    };

    let count = build_cursed_iterator().count();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(
        15,
        y - 2,
        31,
        (count + 3) as i32,
        RGB::named(rltk::WHITE),
        RGB::named(rltk::BLACK),
    );
    ctx.print_color(
        18,
        y - 2,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "Identify Which Item?",
    );
    ctx.print_color(
        18,
        y + count as i32 + 1,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "ESCAPE to cancel",
    );

    let mut equippable: Vec<Entity> = Vec::new();
    for (j, (entity, _item)) in build_cursed_iterator().enumerate() {
        ctx.set(
            17,
            y,
            RGB::named(rltk::WHITE),
            RGB::named(rltk::BLACK),
            rltk::to_cp437('('),
        );
        ctx.set(
            18,
            y,
            RGB::named(rltk::YELLOW),
            RGB::named(rltk::BLACK),
            97 + j as rltk::FontCharType,
        );
        ctx.set(
            19,
            y,
            RGB::named(rltk::WHITE),
            RGB::named(rltk::BLACK),
            rltk::to_cp437(')'),
        );

        ctx.print_color(
            21,
            y,
            get_item_color(&gs.ecs, entity),
            RGB::from_f32(0.0, 0.0, 0.0),
            &get_item_display_name(&gs.ecs, entity),
        );
        equippable.push(entity);
        y += 1;
    }

    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(key) => match key {
            VirtualKeyCode::Escape => (ItemMenuResult::Cancel, None),
            _ => {
                let selection = rltk::letter_to_option(key);
                if selection > -1 && selection < count as i32 {
                    return (
                        ItemMenuResult::Selected,
                        Some(equippable[selection as usize]),
                    );
                }
                (ItemMenuResult::NoResponse, None)
            }
        },
    }
}
