use std::process::id;
use rltk::{BLACK, MAGENTA, RandomNumberGenerator, RGB, to_cp437};
use specs::{Builder, Entities, Entity, World, WorldExt};

use crate::components::{BlocksTile, CombatStats, Item, Monster, Name, Player, Position, Potion, Renderable, Viewshed};
use crate::map::{Map, MAPWIDTH};
use crate::rect::Rect;

const MAX_MONSTERS: i32 = 4;
const MAX_ITEMS: i32 = 2;

pub fn player(ecs: &mut World, player_x: i32, player_y: i32) -> Entity {
    ecs
        .create_entity()
        .with(Position { x: player_x, y: player_y })
        .with(Renderable {
            glyph: to_cp437('@'),
            fg: RGB::named(rltk::YELLOW),
            bg: RGB::named(rltk::BLACK),
            render_order: 0
        })
        .with(Player {})
        .with(Viewshed { visible_tiles: Vec::new(), range: 8, dirty: true })
        .with(Name { name: "Player".to_string() })
        .with(CombatStats { max_hp: 30, hp: 30, defense: 2, power: 5 })
        .build()
}

pub fn random_monster(ecs: &mut World, rng: &mut RandomNumberGenerator, x: i32, y: i32) {
    match rng.roll_dice(1, 2) {
        1 => { orc(ecs, x, y) }
        _ => { goblin(ecs, x, y) }
    }
}

fn orc(ecs: &mut World, x: i32, y: i32) {
    monster(ecs, x, y, to_cp437('o'), "Orc");
}

fn goblin(ecs: &mut World, x: i32, y: i32) {
    monster(ecs, x, y, to_cp437('g'), "Goblin");
}

fn monster<S: ToString>(ecs: &mut World, x: i32, y: i32, glyph: rltk::FontCharType, name: S) {
    ecs
        .create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph,
            fg: RGB::named(rltk::RED),
            bg: RGB::named(rltk::BLACK),
            render_order: 1
        })
        .with(Viewshed { visible_tiles: Vec::new(), range: 8, dirty: true })
        .with(Monster {})
        .with(Name { name: name.to_string() })
        .with(BlocksTile {})
        .with(CombatStats { max_hp: 16, hp: 16, defense: 1, power: 4 })
        .build();
}

pub fn spawn_room(ecs: &mut World, rng: &mut RandomNumberGenerator, room: &Rect) {
   let mut monster_spawn_points= Vec::new();
    let mut item_spawn_points = Vec::new();
    {
        let num_monsters = rng.roll_dice(1, MAX_MONSTERS+2) - 3;
        let num_items = rng.roll_dice(1, MAX_ITEMS+2) - 3;

        for _i in 0..num_monsters {
            let mut added = false;
            while !added {
                let x = (room.x1 + rng.roll_dice(1, i32::abs(room.x2 - room.x1)));
                let y = (room.y1 + rng.roll_dice(1, i32::abs(room.y2 - room.y1)));
                let idx = (y*MAPWIDTH as i32) + x;
                if !monster_spawn_points.contains(&idx) {
                    monster_spawn_points.push(idx);
                    added = true;
                }
            }
        }

        for _i in 0..num_items {
            let mut added = false;
            while !added {
                let x = (room.x1 + rng.roll_dice(1, i32::abs(room.x2 - room.x1)));
                let y = (room.y1 + rng.roll_dice(1, i32::abs(room.y2 - room.y1)));
                let idx = (y*MAPWIDTH as i32) + x;
                if !item_spawn_points.contains(&idx) {
                    item_spawn_points.push(idx);
                    added = true;
                }
            }
        }

        for idx in monster_spawn_points.iter() {
            let x = *idx%MAPWIDTH as i32;
            let y=  *idx / MAPWIDTH as i32;
            random_monster(ecs, rng, x, y);
        }

        for idx in item_spawn_points.iter() {
            let x = *idx % MAPWIDTH as i32;
            let y = *idx / MAPWIDTH as i32;
            health_potion(ecs, x, y);
        }
    }
}

fn health_potion(ecs: &mut World, x: i32, y: i32) {
    ecs
        .create_entity()
        .with(Position{x, y})
        .with(Renderable{
            glyph: to_cp437('¡'),
            fg: RGB::named(MAGENTA),
            bg: RGB::named(BLACK),
            render_order: 2
        })
        .with(Name{name: "Health Potion".to_string()})
        .with(Item{})
        .with(Potion{heal_amount: 8})
        .build();
}