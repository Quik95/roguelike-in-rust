use rltk::{Point, RandomNumberGenerator};
use specs::{Join, World, WorldExt};

use crate::components::{
    AreaOfEffect, Equipped, InBackpack, LootTable, Name, OnDeath, Player, Pools, Position,
};
use crate::effects::{add_effect, aoe_tiles, EffectType, Targets};
use crate::gamelog;
use crate::map::Map;
use crate::player::RunState;
use crate::raws::rawmaster::{find_spell_entity, get_item_drop, spawn_named_item, SpawnType, RAWS};

pub fn delete_the_dead(ecs: &mut World) {
    let mut dead = Vec::new();
    {
        let pools = ecs.read_storage::<Pools>();
        let players = ecs.read_storage::<Player>();
        let entities = ecs.entities();
        let names = ecs.read_storage::<Name>();

        for (entity, pools) in (&entities, &pools).join() {
            if pools.hit_points.current < 1 {
                let player = players.get(entity);
                match player {
                    None => {
                        let victim_name = names.get(entity);
                        if let Some(victim_name) = victim_name {
                            gamelog::Logger::new()
                                .npc_name(&victim_name.name)
                                .append("is dead")
                                .log();
                        }
                        dead.push(entity);
                    }
                    Some(_) => {
                        let mut runstate = ecs.write_resource::<RunState>();
                        *runstate = RunState::GameOver;
                    }
                }
            }
        }
    }

    let mut to_spawn = Vec::new();
    {
        let mut to_drop = Vec::new();
        let entities = ecs.entities();
        let mut equipped = ecs.write_storage::<Equipped>();
        let mut carried = ecs.write_storage::<InBackpack>();
        let mut positions = ecs.write_storage::<Position>();
        let loot_table = ecs.read_storage::<LootTable>();
        let mut rng = ecs.write_resource::<RandomNumberGenerator>();
        for victim in &dead {
            let pos = positions.get(*victim);
            for (entity, equipped) in (&entities, &equipped).join() {
                if equipped.owner == *victim {
                    if let Some(pos) = pos {
                        to_drop.push((entity, pos.clone()));
                    }
                }
            }
            for (entity, backpack) in (&entities, &carried).join() {
                if backpack.owner == *victim {
                    if let Some(pos) = pos {
                        to_drop.push((entity, pos.clone()));
                    }
                }
            }

            if let Some(table) = loot_table.get(*victim) {
                let drop_finder = get_item_drop(&RAWS.lock().unwrap(), &mut rng, &table.table);
                if let Some(tag) = drop_finder {
                    if let Some(pos) = pos {
                        to_spawn.push((tag, pos.clone()));
                    }
                }
            }
        }

        for drop in &to_drop {
            equipped.remove(drop.0);
            carried.remove(drop.0);
            positions
                .insert(drop.0, drop.1.clone())
                .expect("Unable to insert position");
        }
    }

    {
        for drop in &to_spawn {
            spawn_named_item(
                &RAWS.lock().unwrap(),
                ecs,
                &drop.0,
                SpawnType::AtPosition {
                    x: drop.1.x,
                    y: drop.1.y,
                },
            );
        }
    }

    for victim in &dead {
        let death_effects = ecs.read_storage::<OnDeath>();
        if let Some(death_effect) = death_effects.get(*victim) {
            let mut rng = ecs.fetch_mut::<RandomNumberGenerator>();
            for effect in &death_effect.abilities {
                if rng.roll_dice(1, 100) <= (effect.chance * 100.0) as i32 {
                    let map = ecs.fetch::<Map>();
                    if let Some(pos) = ecs.read_storage::<Position>().get(*victim) {
                        let spell_entity = find_spell_entity(ecs, &effect.spell).unwrap();
                        let tile_idx = map.xy_idx(pos.x, pos.y);
                        let target = if let Some(aoe) =
                            ecs.read_storage::<AreaOfEffect>().get(spell_entity)
                        {
                            Targets::Tiles {
                                tiles: aoe_tiles(&map, Point::new(pos.x, pos.y), aoe.radius),
                            }
                        } else {
                            Targets::Tile {
                                tile_idx: tile_idx as i32,
                            }
                        };
                        add_effect(
                            None,
                            EffectType::SpellUse {
                                spell: find_spell_entity(ecs, &effect.spell).unwrap(),
                            },
                            target,
                        );
                    }
                }
            }
        }
    }

    for victim in dead {
        ecs.delete_entity(victim).expect("Unable to delete");
    }
}
