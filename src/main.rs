use itertools::Itertools;
use rltk::{GameState, Point, RandomNumberGenerator};
use specs::prelude::*;

use components::*;
use map::Map;
use map_indexing_system::MapIndexingSystem;
use monster_ai_system::MonsterAI;
use player::RunState;
use RunState::PreRun;
use visibility_system::VisibilitySystem;

use crate::damage_system::DamageSystem;
use crate::gamelog::GameLog;
use crate::inventory_system::{ItemCollectionSystem, ItemDropSystem, ItemUseSystem};
use crate::melee_combat_system::MeleeCombatSystem;
use crate::player::RunState::*;

mod components;
mod map;
mod map_indexing_system;
mod monster_ai_system;
mod player;
mod rect;
mod visibility_system;
mod melee_combat_system;
mod damage_system;
mod gui;
mod gamelog;
mod spawner;
mod inventory_system;

pub struct State {
    ecs: World,
}

impl State {
    fn run_systems(&mut self) {
        let mut vis = VisibilitySystem {};
        vis.run_now(&self.ecs);

        let mut mob = MonsterAI {};
        mob.run_now(&self.ecs);

        let mut mapindex = MapIndexingSystem {};
        mapindex.run_now(&self.ecs);

        let mut melee_combat_system = MeleeCombatSystem {};
        melee_combat_system.run_now(&self.ecs);

        let mut damage_system = DamageSystem {};
        damage_system.run_now(&self.ecs);

        let mut pickup = ItemCollectionSystem {};
        pickup.run_now(&self.ecs);

        let mut potions = ItemUseSystem {};
        potions.run_now(&self.ecs);

        let mut drop_items = ItemDropSystem {};
        drop_items.run_now(&self.ecs);

        self.ecs.maintain();
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut rltk::Rltk) {
        ctx.cls();


        Map::draw_map(&self.ecs, ctx);

        {
            let positions = self.ecs.read_storage::<Position>();
            let renderables = self.ecs.read_storage::<Renderable>();
            let map = self.ecs.fetch::<Map>();

            let data = (&positions, &renderables).join()
                .sorted_by(|&a, &b| b.1.render_order.cmp(&a.1.render_order));
            for (pos, render) in data {
                let idx = Map::xy_idx(pos.x, pos.y);
                if map.visible_tiles[idx] {
                    ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph);
                }
            }
            gui::draw_ui(&self.ecs, ctx);
        }

        let mut newrunstate;
        {
            let runstate = self.ecs.fetch::<RunState>();
            newrunstate = *runstate;
        }

        match newrunstate {
            PreRun => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = AwaitingInput;
            }
            AwaitingInput => {
                newrunstate = Player::player_input(self, ctx);
            }
            PlayerTurn => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = MonsterTurn;
            }
            MonsterTurn => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = AwaitingInput;
            }
            ShowInventory => {
                let result = gui::show_inventory(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let is_ranged = self.ecs.read_storage::<Ranged>();
                        let is_item_ranged = is_ranged.get(item_entity);
                        if let Some(is_item_ranged) = is_item_ranged {
                            newrunstate = RunState::ShowTargeting {range: is_item_ranged.range, item: item_entity};
                        }
                        else {
                            let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                            intent.insert(*self.ecs.fetch::<Entity>(), WantsToUseItem { item: item_entity, target: None }).expect("Unable to insert intent");
                            newrunstate = PlayerTurn;
                        }
                    }
                }
            }
            ShowDropItem => {
                let result = gui::drop_item_menu(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToDropItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToDropItem { item: item_entity }).expect("Unable to insert drop intent");
                        newrunstate = PlayerTurn;
                    }
                }
            },
            ShowTargeting {range, item} => {
                let result=  gui::ranged_target(self, ctx, range);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToUseItem{item, target: result.1}).expect("Unable to insert intent");
                        newrunstate = PlayerTurn;
                    }
                }
            }
        }
        {
            let mut runwriter = self.ecs.write_resource::<RunState>();
            *runwriter = newrunstate;
        }

        damage_system::delete_the_dead(&mut self.ecs);
    }
}

fn main() -> rltk::BError {
    use rltk::RltkBuilder;
    let mut context = RltkBuilder::simple80x50()
        .with_title("Roguelike Tutorial")
        .build()?;
    context.with_post_scanlines(true);

    let mut gs = State {
        ecs: World::new(),
    };

    let mut rng = RandomNumberGenerator::new();

    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<Player>();
    gs.ecs.register::<Viewshed>();
    gs.ecs.register::<Monster>();
    gs.ecs.register::<Name>();
    gs.ecs.register::<BlocksTile>();
    gs.ecs.register::<CombatStats>();
    gs.ecs.register::<WantsToMelee>();
    gs.ecs.register::<SufferDamage>();
    gs.ecs.register::<Item>();
    gs.ecs.register::<ProvidesHealing>();
    gs.ecs.register::<InBackpack>();
    gs.ecs.register::<WantsToPickupItem>();
    gs.ecs.register::<WantsToUseItem>();
    gs.ecs.register::<WantsToDropItem>();
    gs.ecs.register::<Consumable>();
    gs.ecs.register::<Ranged>();
    gs.ecs.register::<InflictsDamage>();
    gs.ecs.register::<AreaOfEffect>();
    gs.ecs.register::<Confusion>();

    let map = Map::new_map_rooms_and_corridors(&mut rng);
    let (player_x, player_y) = map.rooms[0].center();

    let player_entity = spawner::player(&mut gs.ecs, player_x, player_y);

    for room in map.rooms.iter().skip(1) {
        spawner::spawn_room(&mut gs.ecs, &mut rng, room);
    }

    gs.ecs.insert(map);
    gs.ecs.insert(Point::new(player_x, player_y));
    gs.ecs.insert(player_entity);
    gs.ecs.insert(PreRun);
    gs.ecs.insert(GameLog { entries: vec!["Welcome to Rusty Roguelike".to_string()] });
    gs.ecs.insert(rng);

    rltk::main_loop(context, gs)
}
