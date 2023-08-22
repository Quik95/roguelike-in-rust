#![feature(never_type)]

extern crate core;

use rltk::{GameState, Point, RandomNumberGenerator};
use specs::prelude::*;
use specs::saveload::{SimpleMarker, SimpleMarkerAllocator};

use components::*;
use map::Map;
use map_indexing_system::MapIndexingSystem;
use monster_ai_system::MonsterAI;
use player::RunState;
use visibility_system::VisibilitySystem;
use RunState::PreRun;

use crate::animal_ai_system::AnimalAI;
use crate::camera::{render_camera, render_debug_map};
use crate::damage_system::DamageSystem;
use crate::gamelog::GameLog;
use crate::gui::draw_ui;
use crate::inventory_system::{
    ItemCollectionSystem, ItemDropSystem, ItemRemoveSystem, ItemUseSystem,
};
use crate::map::dungeon::{
    freeze_level_entities, level_transition, thaw_level_entities, MasterDungeonMap,
};
use crate::melee_combat_system::MeleeCombatSystem;
use crate::player::RunState::*;

mod animal_ai_system;
pub mod bystander_ai_system;
mod camera;
mod components;
mod damage_system;
mod gamelog;
mod gamesystem;
mod gui;
mod hunger_system;
mod inventory_system;
mod map;
mod map_builders;
mod map_indexing_system;
mod melee_combat_system;
mod menu;
mod monster_ai_system;
mod particle_system;
mod player;
mod random_table;
mod raws;
mod rect;
mod rex_assets;
mod saveload_system;
mod spawner;
mod trigger_system;
mod visibility_system;

const SHOW_MAPGEN_VISUALIZER: bool = true;

pub struct State {
    ecs: World,
    mapgen_next_state: Option<RunState>,
    mapgen_history: Vec<Map>,
    mapgen_index: usize,
    mapgen_timer: f32,
}

impl State {
    fn run_systems(&mut self) {
        let mut vis = VisibilitySystem {};
        vis.run_now(&self.ecs);

        let mut mob = MonsterAI {};
        mob.run_now(&self.ecs);

        let mut bystander = bystander_ai_system::BystanderAI {};
        bystander.run_now(&self.ecs);

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

        let mut item_remove = ItemRemoveSystem {};
        item_remove.run_now(&self.ecs);

        let mut particles = particle_system::ParticleSpawnSystem {};
        particles.run_now(&self.ecs);

        let mut hunger = hunger_system::HungerSystem {};
        hunger.run_now(&self.ecs);

        let mut triggers = trigger_system::TriggerSystem {};
        triggers.run_now(&self.ecs);

        let mut animal_ai = AnimalAI {};
        animal_ai.run_now(&self.ecs);

        self.ecs.maintain();
    }

    fn generate_world_map(&mut self, new_depth: i32, offset: i32) {
        self.mapgen_index = 0;
        self.mapgen_timer = 0.0;
        self.mapgen_history.clear();
        let map_building_info = level_transition(&mut self.ecs, new_depth, offset);
        if let Some(history) = map_building_info {
            self.mapgen_history = history;
        } else {
            thaw_level_entities(&mut self.ecs);
        }
    }

    fn entities_to_remove_on_level_change(&mut self) -> Vec<Entity> {
        let entities = self.ecs.entities();
        let player = self.ecs.read_storage::<Player>();
        let backpack = self.ecs.read_storage::<InBackpack>();
        let player_entity = self.ecs.fetch::<Entity>();
        let equipped = self.ecs.read_storage::<Equipped>();

        let mut to_delete: Vec<Entity> = Vec::new();
        for entity in entities.join() {
            let mut should_delete = true;

            // Don't delete the player
            let p = player.get(entity);
            if let Some(_p) = p {
                should_delete = false;
            }

            // Don't delete the player's equipment
            let bp = backpack.get(entity);
            if let Some(bp) = bp {
                if bp.owner == *player_entity {
                    should_delete = false;
                }
            }

            let eq = equipped.get(entity);
            if let Some(eq) = eq {
                if eq.owner == *player_entity {
                    should_delete = false;
                }
            }

            if should_delete {
                to_delete.push(entity);
            }
        }

        to_delete
    }

    fn goto_level(&mut self, offset: i32) {
        freeze_level_entities(&mut self.ecs);

        // Build a new map and place the player
        let current_depth = self.ecs.fetch::<Map>().depth;
        self.generate_world_map(current_depth + offset, offset);

        // Notify the player
        let mut gamelog = self.ecs.fetch_mut::<gamelog::GameLog>();
        gamelog.entries.push("You change level.".to_string());
    }

    fn game_over_cleanup(&mut self) {
        let mut to_delete = Vec::new();
        for e in self.ecs.entities().join() {
            to_delete.push(e);
        }

        for del in to_delete.iter() {
            self.ecs.delete_entity(*del).expect("Deletion failed");
        }

        {
            let player_entity = spawner::player(&mut self.ecs, 0, 0);
            let mut player_entity_writer = self.ecs.write_resource::<Entity>();
            *player_entity_writer = player_entity;
        }

        self.ecs.insert(MasterDungeonMap::default());

        self.generate_world_map(1, 0);
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut rltk::Rltk) {
        let mut newrunstate;
        {
            let runstate = self.ecs.fetch::<RunState>();
            newrunstate = *runstate;
        }

        ctx.cls();
        particle_system::cull_dead_particles(&mut self.ecs, ctx);

        match newrunstate {
            GameOver { .. } => {}
            _ => {
                render_camera(&self.ecs, ctx);
                draw_ui(&self.ecs, ctx);
            }
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
                match *self.ecs.fetch::<RunState>() {
                    MagicMapReveal { .. } => newrunstate = MagicMapReveal { row: 0 },
                    _ => newrunstate = MonsterTurn,
                }
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
                            newrunstate = ShowTargeting {
                                range: is_item_ranged.range,
                                item: item_entity,
                            };
                        } else {
                            let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                            intent
                                .insert(
                                    *self.ecs.fetch::<Entity>(),
                                    WantsToUseItem {
                                        item: item_entity,
                                        target: None,
                                    },
                                )
                                .expect("Unable to insert intent");
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
                        intent
                            .insert(
                                *self.ecs.fetch::<Entity>(),
                                WantsToDropItem { item: item_entity },
                            )
                            .expect("Unable to insert drop intent");
                        newrunstate = PlayerTurn;
                    }
                }
            }
            ShowTargeting { range, item } => {
                let result = gui::ranged_target(self, ctx, range);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                        intent
                            .insert(
                                *self.ecs.fetch::<Entity>(),
                                WantsToUseItem {
                                    item,
                                    target: result.1,
                                },
                            )
                            .expect("Unable to insert intent");
                        newrunstate = PlayerTurn;
                    }
                }
            }
            MainMenu { .. } => {
                let result = menu::main_menu(self, ctx);
                match result {
                    gui::MainMenuResult::NoSelection { selected } => {
                        newrunstate = MainMenu {
                            menu_selection: selected,
                        }
                    }
                    gui::MainMenuResult::Selected { selected } => match selected {
                        gui::MainMenuSelection::NewGame => newrunstate = PreRun,
                        gui::MainMenuSelection::LoadGame => {
                            saveload_system::load_game(&mut self.ecs);
                            newrunstate = AwaitingInput;
                            saveload_system::delete_save();
                        }
                        gui::MainMenuSelection::Quit => ctx.quit(),
                    },
                }
            }
            SaveGame => {
                saveload_system::save_game(&mut self.ecs);
                newrunstate = MainMenu {
                    menu_selection: gui::MainMenuSelection::LoadGame,
                };
            }
            NextLevel => {
                self.goto_level(1);
                self.mapgen_next_state = Some(PreRun);
                newrunstate = MapGeneration;
            }
            PreviousLevel => {
                self.goto_level(-1);
                self.mapgen_next_state = Some(PreRun);
                newrunstate = MapGeneration;
            }
            ShowRemoveItem => {
                let result = gui::remove_item_menu(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToRemoveItem>();
                        intent
                            .insert(
                                *self.ecs.fetch::<Entity>(),
                                WantsToRemoveItem { item: item_entity },
                            )
                            .expect("Unable to insert drop intent");
                        newrunstate = PlayerTurn;
                    }
                }
            }
            GameOver => {
                let result = gui::game_over(ctx);
                match result {
                    gui::GameOverResult::NoSelection => {}
                    gui::GameOverResult::QuitToMenu => {
                        self.game_over_cleanup();
                        newrunstate = MainMenu {
                            menu_selection: gui::MainMenuSelection::NewGame,
                        };
                    }
                }
            }
            MagicMapReveal { row } => {
                let mut map = self.ecs.fetch_mut::<Map>();
                for x in 0..map.width {
                    let idx = map.xy_idx(x, row);
                    map.revealed_tiles[idx] = true;
                }
                if row == map.height - 1 {
                    newrunstate = MonsterTurn;
                } else {
                    newrunstate = MagicMapReveal { row: row + 1 };
                }
            }
            MapGeneration => {
                if !SHOW_MAPGEN_VISUALIZER {
                    newrunstate = self.mapgen_next_state.unwrap();
                } else {
                    ctx.cls();
                    if self.mapgen_index < self.mapgen_history.len() {
                        render_debug_map(&self.mapgen_history[self.mapgen_index], ctx);
                    }

                    self.mapgen_timer += ctx.frame_time_ms;
                    if self.mapgen_timer > 100.0 {
                        self.mapgen_timer = 0.0;
                        self.mapgen_index += 1;
                        if self.mapgen_index >= self.mapgen_history.len() {
                            newrunstate = self.mapgen_next_state.unwrap();
                        }
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
    let mut context = RltkBuilder::simple(80, 60)
        .unwrap()
        .with_title("Roguelike Tutorial")
        .build()?;
    context.with_post_scanlines(true);

    let mut gs = State {
        ecs: World::new(),
        mapgen_next_state: Some(MainMenu {
            menu_selection: gui::MainMenuSelection::NewGame,
        }),
        mapgen_index: 0,
        mapgen_history: Vec::new(),
        mapgen_timer: 0.0,
    };

    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<Player>();
    gs.ecs.register::<Viewshed>();
    gs.ecs.register::<Monster>();
    gs.ecs.register::<Name>();
    gs.ecs.register::<BlocksTile>();
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
    gs.ecs.register::<SimpleMarker<SerializeMe>>();
    gs.ecs.register::<SerializationHelper>();
    gs.ecs.register::<Equippable>();
    gs.ecs.register::<Equipped>();
    gs.ecs.register::<MeleePowerBonus>();
    gs.ecs.register::<DefenseBonus>();
    gs.ecs.register::<WantsToRemoveItem>();
    gs.ecs.register::<ParticleLifetime>();
    gs.ecs.register::<HungerClock>();
    gs.ecs.register::<ProvidesFood>();
    gs.ecs.register::<MagicMapper>();
    gs.ecs.register::<Hidden>();
    gs.ecs.register::<EntryTrigger>();
    gs.ecs.register::<EntityMoved>();
    gs.ecs.register::<SingleActivation>();
    gs.ecs.register::<BlocksVisibility>();
    gs.ecs.register::<Door>();
    gs.ecs.register::<Bystander>();
    gs.ecs.register::<Vendor>();
    gs.ecs.register::<Quips>();
    gs.ecs.register::<Attributes>();
    gs.ecs.register::<Skills>();
    gs.ecs.register::<Pools>();
    gs.ecs.register::<MeleeWeapon>();
    gs.ecs.register::<Wearable>();
    gs.ecs.register::<NaturalAttackDefense>();
    gs.ecs.register::<LootTable>();
    gs.ecs.register::<Carnivore>();
    gs.ecs.register::<Herbivore>();
    gs.ecs.register::<OtherLevelPosition>();
    gs.ecs.register::<DMSerializationHelper>();
    gs.ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());

    raws::load_raws();

    gs.ecs.insert(MasterDungeonMap::default());
    gs.ecs.insert(Map::new(1, 64, 64, "New Map"));
    gs.ecs.insert(Point::new(0, 0));
    gs.ecs.insert(RandomNumberGenerator::new());
    let player_entity = spawner::player(&mut gs.ecs, 0, 0);
    gs.ecs.insert(player_entity);
    gs.ecs.insert(MapGeneration {});
    gs.ecs.insert(GameLog {
        entries: vec!["Welcome to Rusty Roguelike".to_string()],
    });
    gs.ecs.insert(particle_system::ParticleBuilder::new());
    gs.ecs.insert(rex_assets::RexAssets::new());

    gs.generate_world_map(1, 0);

    rltk::main_loop(context, gs)
}
