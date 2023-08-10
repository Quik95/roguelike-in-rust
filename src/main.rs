use rltk::{GameState, Point, RandomNumberGenerator};
use specs::prelude::*;
use specs::saveload::{SimpleMarker, SimpleMarkerAllocator};

use components::*;
use map::Map;
use map_indexing_system::MapIndexingSystem;
use monster_ai_system::MonsterAI;
use player::RunState;
use RunState::PreRun;
use visibility_system::VisibilitySystem;

use crate::camera::{render_camera, render_debug_map};
use crate::damage_system::DamageSystem;
use crate::gamelog::GameLog;
use crate::gui::draw_ui;
use crate::inventory_system::{ItemCollectionSystem, ItemDropSystem, ItemRemoveSystem, ItemUseSystem};
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
mod menu;
mod saveload_system;
mod random_table;
mod particle_system;
mod hunger_system;
mod rex_assets;
mod trigger_system;
mod map_builders;
mod camera;

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

        self.ecs.maintain();
    }

    fn generate_world_map(&mut self, new_depth: i32) {
        self.mapgen_index = 0;
        self.mapgen_timer = 0.0;
        self.mapgen_history.clear();

        let mut rng = self.ecs.write_resource::<RandomNumberGenerator>();
        let mut builder = map_builders::random_builder(new_depth, &mut rng, 80, 50);
        builder.build_map(&mut rng);
        std::mem::drop(rng);

        self.mapgen_history = builder.build_data.history.clone();
        let player_start;
        {
            let mut worldmap_resource = self.ecs.write_resource::<Map>();
            *worldmap_resource = builder.build_data.map.clone();
            player_start = builder.build_data.starting_position.as_mut().unwrap().clone();
        }

        builder.spawn_entities(&mut self.ecs);

        let (player_x, player_y) = (player_start.x, player_start.y);
        let mut player_position = self.ecs.write_resource::<Point>();
        *player_position = Point::new(player_x, player_y);

        let mut position_components = self.ecs.write_storage::<Position>();
        let player_entity = self.ecs.fetch::<Entity>();
        let player_pos_comp = position_components.get_mut(*player_entity);
        if let Some(player_pos_comp) = player_pos_comp {
            player_pos_comp.x = player_x;
            player_pos_comp.y = player_y;
        }

        let mut viewshed_componenets = self.ecs.write_storage::<Viewshed>();
        let vs = viewshed_componenets.get_mut(*player_entity);
        if let Some(vs) = vs {
            vs.dirty = true;
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

    fn goto_next_level(&mut self) {
        let to_delete = self.entities_to_remove_on_level_change();
        for target in to_delete {
            self.ecs.delete_entity(target).expect("Unable to delete");
        }

        let current_depth;
        {
            let worldmap_resource = self.ecs.write_resource::<Map>();
            current_depth = worldmap_resource.depth;
        }
        self.generate_world_map(current_depth + 1);

        let player_entity = self.ecs.fetch::<Entity>();
        let mut gamelog = self.ecs.fetch_mut::<GameLog>();

        gamelog.entries.push("You descend to the next level, and take a moment to heal.".to_string());

        let mut player_health_store = self.ecs.write_storage::<CombatStats>();
        let player_health = player_health_store.get_mut(*player_entity);
        if let Some(player_health) = player_health {
            player_health.hp = i32::max(player_health.hp, player_health.max_hp / 2);
        }
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

        self.generate_world_map(1);
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
                            newrunstate = ShowTargeting { range: is_item_ranged.range, item: item_entity };
                        } else {
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
            }
            ShowTargeting { range, item } => {
                let result = gui::ranged_target(self, ctx, range);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToUseItem { item, target: result.1 }).expect("Unable to insert intent");
                        newrunstate = PlayerTurn;
                    }
                }
            }
            MainMenu { .. } => {
                let result = menu::main_menu(self, ctx);
                match result {
                    gui::MainMenuResult::NoSelection { selected } => newrunstate = MainMenu { menu_selection: selected },
                    gui::MainMenuResult::Selected { selected } => {
                        match selected {
                            gui::MainMenuSelection::NewGame => newrunstate = PreRun,
                            gui::MainMenuSelection::LoadGame => {
                                saveload_system::load_game(&mut self.ecs);
                                newrunstate = AwaitingInput;
                                saveload_system::delete_save();
                            }
                            gui::MainMenuSelection::Quit => ctx.quit(),
                        }
                    }
                }
            }
            SaveGame => {
                saveload_system::save_game(&mut self.ecs);
                newrunstate = MainMenu { menu_selection: gui::MainMenuSelection::LoadGame };
            }
            NextLevel => {
                self.goto_next_level();
                newrunstate = PreRun;
            }
            ShowRemoveItem => {
                let result = gui::remove_item_menu(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => newrunstate = AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToRemoveItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToRemoveItem { item: item_entity }).expect("Unable to insert drop intent");
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
                        newrunstate = MainMenu { menu_selection: gui::MainMenuSelection::NewGame };
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
    let mut context = RltkBuilder::simple80x50()
        .with_title("Roguelike Tutorial")
        .build()?;
    context.with_post_scanlines(true);

    let mut gs = State {
        ecs: World::new(),
        mapgen_next_state: Some(MainMenu { menu_selection: gui::MainMenuSelection::NewGame }),
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


    gs.ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());
    gs.ecs.insert(Map::new(1, 64, 64));
    gs.ecs.insert(Point::new(0, 0));
    gs.ecs.insert(RandomNumberGenerator::new());
    let player_entity = spawner::player(&mut gs.ecs, 0, 0);
    gs.ecs.insert(player_entity);
    gs.ecs.insert(MapGeneration {});
    gs.ecs.insert(GameLog { entries: vec!["Welcome to Rusty Roguelike".to_string()] });
    gs.ecs.insert(particle_system::ParticleBuilder::new());
    gs.ecs.insert(rex_assets::RexAssets::new());

    gs.generate_world_map(1);

    rltk::main_loop(context, gs)
}
