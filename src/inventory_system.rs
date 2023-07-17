use specs::{Entities, Entity, Join, ReadExpect, ReadStorage, System, WriteExpect, WriteStorage};

use crate::components::{AreaOfEffect, CombatStats, Confusion, Consumable, InBackpack, InflictsDamage, Name, Position, ProvidesHealing, SufferDamage, WantsToDropItem, WantsToPickupItem, WantsToUseItem};
use crate::gamelog::GameLog;
use crate::map::Map;

pub struct ItemCollectionSystem {}

impl<'a> System<'a> for ItemCollectionSystem {
    type SystemData = (
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        WriteStorage<'a, WantsToPickupItem>,
        WriteStorage<'a, Position>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, InBackpack>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (player_entity, mut gamelog, mut wants_pickup, mut positions, names, mut backpack) = data;

        for pickup in wants_pickup.join() {
            positions.remove(pickup.item);
            backpack.insert(pickup.item, InBackpack { owner: pickup.collected_by }).expect("Unable to insert backpack entry");

            if pickup.collected_by == *player_entity {
                gamelog.entries.push(format!("You pick up the {}.", names.get(pickup.item).unwrap().name));
            }
        }

        wants_pickup.clear();
    }
}

pub struct ItemUseSystem {}

impl<'a> System<'a> for ItemUseSystem {
    type SystemData = (
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        Entities<'a>,
        WriteStorage<'a, WantsToUseItem>,
        ReadStorage<'a, Name>,
        ReadStorage<'a, ProvidesHealing>,
        ReadStorage<'a, InflictsDamage>,
        WriteStorage<'a, CombatStats>,
        ReadStorage<'a, Consumable>,
        ReadExpect<'a, Map>,
        WriteStorage<'a, SufferDamage>,
        ReadStorage<'a, AreaOfEffect>,
        WriteStorage<'a, Confusion>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (player_entity, mut gamelog, entities, mut wants_item, names, healing, inflict_damage, mut combat_stats, consumables, map, mut suffer_damage, aoe, mut confused) = data;

        for (entity, useitem) in (&entities, &wants_item).join() {
            let mut used_item = true;
            let mut targets = Vec::new();
            match useitem.target {
                None => { targets.push(*player_entity) }
                Some(target) => {
                    let area_effect = aoe.get(useitem.item);
                    match area_effect {
                        None => {
                            let idx = Map::xy_idx(target.x, target.y);
                            for mob in map.tile_content[idx].iter() {
                                targets.push(*mob);
                            }
                        }
                        Some(area_effect) => {
                            let mut blast_tiles = rltk::field_of_view(target, area_effect.radius, &*map);
                            blast_tiles.retain(|p| p.x > 0 && p.x < map.width - 1 && p.y > 0 && p.y < map.height - 1);
                            for tile_idx in blast_tiles.iter() {
                                let idx = Map::xy_idx(tile_idx.x, tile_idx.y);
                                for mob in map.tile_content[idx].iter() {
                                    targets.push(*mob);
                                }
                            }
                        }
                    }
                }
            }

            let consumable = consumables.get(useitem.item);
            match consumable {
                None => {}
                Some(_) => {
                    entities.delete(useitem.item).expect("Delete failed");
                }
            }

            let item_heals = healing.get(useitem.item);
            if let Some(healer) = item_heals {
                for target in targets.iter() {
                    let stats = combat_stats.get_mut(*target);
                    if let Some(stats) = stats {
                        stats.hp = i32::min(stats.max_hp, stats.hp + healer.heal_amount);
                        if entity == *player_entity {
                            gamelog.entries.push(format!("You drink the {}, healing {} hp.", names.get(useitem.item).unwrap().name, healer.heal_amount));
                        }
                    }
                }
            }

            let item_damages = inflict_damage.get(useitem.item);
            if let Some(damage) = item_damages {
                used_item = false;
                for mob in targets.iter() {
                    SufferDamage::new_damage(&mut suffer_damage, *mob, damage.damage);
                    if entity == *player_entity {
                        let mob_name = names.get(*mob).unwrap();
                        let item_name = names.get(useitem.item).unwrap();
                        gamelog.entries.push(format!("You use {} on {}, inflicting {} hp.", item_name.name, mob_name.name, damage.damage));
                    }
                    used_item = true;
                }
            }

            let mut add_confusion = Vec::new();
            {
                let causes_confusion = confused.get(useitem.item);
                if let Some(confusion) = causes_confusion {
                    used_item = false;
                    for mob in targets.iter() {
                        add_confusion.push((*mob, confusion.turns));
                        if entity == *player_entity {
                            let mob_name = names.get(*mob).unwrap();
                            let item_name = names.get(useitem.item).unwrap();
                            gamelog.entries.push(format!("You use {} on {}, confusing them.", item_name.name, mob_name.name));
                        }
                    }
                }
            }
            for mob in add_confusion.iter() {
                confused.insert(mob.0, Confusion { turns: mob.1 }).expect("Unable to insert.");
            }
        }

        wants_item.clear();
    }
}

pub struct ItemDropSystem {}

impl<'a> System<'a> for ItemDropSystem {
    type SystemData = (
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        Entities<'a>,
        WriteStorage<'a, WantsToDropItem>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, InBackpack>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (player_entity, mut gamelog, entities, mut wants_drop, names, mut positions, mut backpack) = data;

        for (entity, to_drop) in (&entities, &wants_drop).join() {
            let mut dropper_pos = Position { x: 0, y: 0 };
            {
                let dropped_pos = positions.get(entity).unwrap();
                dropper_pos.x = dropped_pos.x;
                dropper_pos.y = dropped_pos.y;
            }
            positions.insert(to_drop.item, Position { x: dropper_pos.x, y: dropper_pos.y }).expect("Unable to insert position");
            backpack.remove(to_drop.item);

            if entity == *player_entity {
                gamelog.entries.push(format!("You drop the {}.", names.get(to_drop.item).unwrap().name));
            }
        }

        wants_drop.clear();
    }
}