#![feature(never_type)]

extern crate core;

use rltk::{GameState, Point, RandomNumberGenerator};
use specs::prelude::*;
use specs::saveload::{SimpleMarker, SimpleMarkerAllocator};

use components::{
    ApplyMove, ApplyTeleport, AreaOfEffect, Attributes, BlocksTile, BlocksVisibility, Chasing,
    Confusion, Consumable, DMSerializationHelper, DefenseBonus, Door, EntityMoved, EntryTrigger,
    EquipmentChanged, Equippable, Equipped, Faction, Hidden, HungerClock, IdentifiedItem,
    InBackpack, InflictsDamage, Initiative, Item, LightSource, LootTable, MagicItem, MagicMapper,
    MeleePowerBonus, MeleeWeapon, MoveMode, MyTurn, Name, NaturalAttackDefense, ObfuscatedName,
    OtherLevelPosition, ParticleLifetime, Player, Pools, Position, ProvidesFood, ProvidesHealing,
    Quips, Ranged, Renderable, SerializationHelper, SerializeMe, SingleActivation, Skills,
    SpawnParticleBurst, SpawnParticleLine, TeleportTo, Vendor, Viewshed, WantsToApproach,
    WantsToDropItem, WantsToFlee, WantsToMelee, WantsToPickupItem, WantsToRemoveItem,
    WantsToUseItem, Wearable,
};
use map::Map;
use map_indexing_system::MapIndexingSystem;
use player::RunState;
use visibility_system::VisibilitySystem;
use RunState::PreRun;

use crate::camera::{render_camera, render_debug_map};
use crate::gamelog::GameLog;
use crate::gui::{draw_ui, show_cheat_mode, show_vendor_menu, CheatMenuResult, VendorResult};
use crate::inventory_system::{
    ItemCollectionSystem, ItemDropSystem, ItemRemoveSystem, ItemUseSystem,
};
use crate::lightning_system::LightingSystem;
use crate::map::dungeon::{
    freeze_level_entities, level_transition, thaw_level_entities, MasterDungeonMap,
};
use crate::melee_combat_system::MeleeCombatSystem;
use crate::player::RunState::{
    AwaitingInput, GameOver, MagicMapReveal, MainMenu, MapGeneration, NextLevel, PreviousLevel,
    SaveGame, ShowCheatMenu, ShowDropItem, ShowInventory, ShowRemoveItem, ShowTargeting, Ticking,
    TownPortal,
};
use crate::player::VendorMode;
use crate::raws::rawmaster::{spawn_named_item, SpawnType, RAWS};

mod ai;
mod camera;
mod cave_decorator;
mod components;
mod damage_system;
pub mod effects;
mod gamelog;
mod gamesystem;
mod gui;
mod hunger_system;
mod inventory_system;
mod lightning_system;
mod map;
mod map_builders;
mod map_indexing_system;
mod melee_combat_system;
mod menu;
mod movement_system;
mod particle_system;
mod player;
mod random_table;
mod raws;
mod rect;
mod rex_assets;
mod saveload_system;
mod spatial;
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
        let mut mapindex = MapIndexingSystem {};
        mapindex.run_now(&self.ecs);

        let mut vis = VisibilitySystem {};
        vis.run_now(&self.ecs);

        let mut encumbrance = ai::EncumbranceSystem {};
        encumbrance.run_now(&self.ecs);

        let mut initiative = ai::InitiativeSystem {};
        initiative.run_now(&self.ecs);

        let mut turnstatus = ai::TurnStatusSystem {};
        turnstatus.run_now(&self.ecs);

        let mut quipper = ai::QuipSystem {};
        quipper.run_now(&self.ecs);

        let mut adjacent = ai::AdjacentAI {};
        adjacent.run_now(&self.ecs);

        let mut visible = ai::VisibleAI {};
        visible.run_now(&self.ecs);

        let mut approach = ai::ApproachAI {};
        approach.run_now(&self.ecs);

        let mut flee = ai::FleeAI {};
        flee.run_now(&self.ecs);

        let mut chase = ai::ChaseAI {};
        chase.run_now(&self.ecs);

        let mut defaultmove = ai::DefaultMoveAI {};
        defaultmove.run_now(&self.ecs);

        let mut moving = movement_system::MovementSystem {};
        moving.run_now(&self.ecs);

        let mut triggers = trigger_system::TriggerSystem {};
        triggers.run_now(&self.ecs);

        let mut melee = MeleeCombatSystem {};
        melee.run_now(&self.ecs);

        let mut pickup = ItemCollectionSystem {};
        pickup.run_now(&self.ecs);

        let mut itemequip = inventory_system::ItemEquipOnUse {};
        itemequip.run_now(&self.ecs);

        let mut itemuse = ItemUseSystem {};
        itemuse.run_now(&self.ecs);

        let mut item_id = inventory_system::ItemIdentificationSystem {};
        item_id.run_now(&self.ecs);

        let mut drop_items = ItemDropSystem {};
        drop_items.run_now(&self.ecs);

        let mut item_remove = ItemRemoveSystem {};
        item_remove.run_now(&self.ecs);

        let mut hunger = hunger_system::HungerSystem {};
        hunger.run_now(&self.ecs);

        effects::run_effects_queue(&self.ecs);
        let mut particles = particle_system::ParticleSpawnSystem {};
        particles.run_now(&self.ecs);

        let mut lighting = LightingSystem {};
        lighting.run_now(&self.ecs);

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
            thaw_level_entities(&self.ecs);
        }
    }

    fn goto_level(&mut self, offset: i32) {
        freeze_level_entities(&self.ecs);

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

        for del in &to_delete {
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
                            newrunstate = Ticking;
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
                        newrunstate = Ticking;
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
                        newrunstate = Ticking;
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
                        newrunstate = Ticking;
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
                    newrunstate = Ticking;
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
            ShowCheatMenu => {
                let result = show_cheat_mode(self, ctx);
                match result {
                    CheatMenuResult::Cancel => newrunstate = AwaitingInput,
                    CheatMenuResult::NoResponse => {}
                    CheatMenuResult::TeleportToExit => {
                        self.goto_level(1);
                        self.mapgen_next_state = Some(PreRun);
                        newrunstate = MapGeneration;
                    }
                    CheatMenuResult::Heal => {
                        let player = self.ecs.fetch::<Entity>();
                        let mut pools = self.ecs.write_storage::<Pools>();
                        let player_pools = pools.get_mut(*player).unwrap();
                        player_pools.hit_points.current = player_pools.hit_points.max;
                        newrunstate = RunState::AwaitingInput;
                    }
                    CheatMenuResult::Reveal => {
                        let mut map = self.ecs.fetch_mut::<Map>();
                        for v in &mut map.revealed_tiles {
                            *v = true;
                        }
                        newrunstate = RunState::AwaitingInput;
                    }
                    CheatMenuResult::GodMode => {
                        let player = self.ecs.fetch::<Entity>();
                        let mut pools = self.ecs.write_storage::<Pools>();
                        let player_pools = pools.get_mut(*player).unwrap();
                        player_pools.god_mode = true;
                        newrunstate = RunState::AwaitingInput;
                    }
                }
            }
            Ticking => {
                while newrunstate == RunState::Ticking {
                    self.run_systems();
                    self.ecs.maintain();
                    match *self.ecs.fetch::<RunState>() {
                        AwaitingInput => newrunstate = AwaitingInput,
                        MagicMapReveal { .. } => newrunstate = MagicMapReveal { row: 0 },
                        TownPortal => newrunstate = RunState::TownPortal,
                        RunState::TeleportingToOtherLevel { x, y, depth } => {
                            newrunstate = RunState::TeleportingToOtherLevel { x, y, depth }
                        }
                        _ => newrunstate = Ticking,
                    }
                }
            }
            RunState::ShowVendor { vendor, mode } => {
                let result = show_vendor_menu(self, ctx, vendor, mode);
                match result.0 {
                    VendorResult::Cancel => newrunstate = RunState::AwaitingInput,
                    VendorResult::NoResponse => {}
                    VendorResult::Sell => {
                        let price = self
                            .ecs
                            .read_storage::<Item>()
                            .get(result.1.unwrap())
                            .unwrap()
                            .base_value
                            * 0.8;
                        self.ecs
                            .write_storage::<Pools>()
                            .get_mut(*self.ecs.fetch::<Entity>())
                            .unwrap()
                            .gold += price;
                        self.ecs
                            .delete_entity(result.1.unwrap())
                            .expect("Unable to delete");
                    }
                    VendorResult::Buy => {
                        let tag = result.2.unwrap();
                        let price = result.3.unwrap();
                        let mut pools = self.ecs.write_storage::<Pools>();
                        let player_entity = self.ecs.fetch::<Entity>();
                        let mut identified = self.ecs.write_storage::<IdentifiedItem>();
                        identified
                            .insert(*player_entity, IdentifiedItem { name: tag.clone() })
                            .expect("Unable to insert");
                        std::mem::drop(identified);
                        let player_pools = pools.get_mut(*player_entity).unwrap();
                        std::mem::drop(player_entity);

                        if player_pools.gold >= price {
                            player_pools.gold -= price;
                            std::mem::drop(pools);
                            let player_entity = *self.ecs.fetch::<Entity>();
                            spawn_named_item(
                                &RAWS.lock().unwrap(),
                                &mut self.ecs,
                                &tag,
                                SpawnType::Carried { by: player_entity },
                            );
                        }
                    }
                    VendorResult::BuyMode => {
                        newrunstate = RunState::ShowVendor {
                            vendor,
                            mode: VendorMode::Buy,
                        }
                    }
                    VendorResult::SellMode => {
                        newrunstate = RunState::ShowVendor {
                            vendor,
                            mode: VendorMode::Sell,
                        }
                    }
                }
            }
            RunState::TownPortal => {
                spawner::spawn_town_portal(&mut self.ecs);

                let map_depth = self.ecs.fetch::<Map>().depth;
                let destination_offset = 0 - (map_depth - 1);
                self.goto_level(destination_offset);
                self.mapgen_next_state = Some(RunState::PreRun);
                newrunstate = RunState::MapGeneration;
            }
            RunState::TeleportingToOtherLevel { x, y, depth } => {
                self.goto_level(depth - 1);
                let player_entity = self.ecs.fetch::<Entity>();
                if let Some(pos) = self.ecs.write_storage::<Position>().get_mut(*player_entity) {
                    pos.x = x;
                    pos.y = y;
                }
                let mut ppos = self.ecs.fetch_mut::<Point>();
                ppos.x = x;
                ppos.y = y;
                self.mapgen_next_state = Some(RunState::PreRun);
                newrunstate = RunState::MapGeneration;
            }
        }
        {
            let mut runwriter = self.ecs.write_resource::<RunState>();
            *runwriter = newrunstate;
        }

        damage_system::delete_the_dead(&mut self.ecs);
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    use rltk::RltkBuilder;
    let mut context = RltkBuilder::simple(80, 60)
        .unwrap()
        .with_title("Roguelike Tutorial")
        .with_fps_cap(60.0)
        .build()
        .expect("Failed to construct a builder.");
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
    gs.ecs.register::<Name>();
    gs.ecs.register::<BlocksTile>();
    gs.ecs.register::<WantsToMelee>();
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
    gs.ecs.register::<Quips>();
    gs.ecs.register::<Attributes>();
    gs.ecs.register::<Skills>();
    gs.ecs.register::<Pools>();
    gs.ecs.register::<MeleeWeapon>();
    gs.ecs.register::<Wearable>();
    gs.ecs.register::<NaturalAttackDefense>();
    gs.ecs.register::<LootTable>();
    gs.ecs.register::<OtherLevelPosition>();
    gs.ecs.register::<DMSerializationHelper>();
    gs.ecs.register::<LightSource>();
    gs.ecs.register::<Initiative>();
    gs.ecs.register::<MyTurn>();
    gs.ecs.register::<Faction>();
    gs.ecs.register::<WantsToApproach>();
    gs.ecs.register::<WantsToFlee>();
    gs.ecs.register::<MoveMode>();
    gs.ecs.register::<Chasing>();
    gs.ecs.register::<EquipmentChanged>();
    gs.ecs.register::<Vendor>();
    gs.ecs.register::<components::TownPortal>();
    gs.ecs.register::<TeleportTo>();
    gs.ecs.register::<ApplyMove>();
    gs.ecs.register::<ApplyTeleport>();
    gs.ecs.register::<MagicItem>();
    gs.ecs.register::<ObfuscatedName>();
    gs.ecs.register::<IdentifiedItem>();
    gs.ecs.register::<SpawnParticleLine>();
    gs.ecs.register::<SpawnParticleBurst>();
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

    rltk::main_loop(context, gs).expect("Failed to run main loop");

    Ok(())
}
