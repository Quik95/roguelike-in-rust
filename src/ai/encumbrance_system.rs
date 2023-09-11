use std::collections::HashMap;

use specs::{Entities, Entity, Join, ReadExpect, ReadStorage, System, WriteExpect, WriteStorage};

use crate::components::{
    AttributeBonus, Attributes, EquipmentChanged, Equipped, InBackpack, Item, Pools, Slow,
    StatusEffect,
};
use crate::gamelog::GameLog;

pub struct EncumbranceSystem {}

impl<'a> System<'a> for EncumbranceSystem {
    type SystemData = (
        WriteStorage<'a, EquipmentChanged>,
        Entities<'a>,
        ReadStorage<'a, Item>,
        ReadStorage<'a, InBackpack>,
        ReadStorage<'a, Equipped>,
        WriteStorage<'a, Pools>,
        WriteStorage<'a, Attributes>,
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        ReadStorage<'a, AttributeBonus>,
        ReadStorage<'a, StatusEffect>,
        ReadStorage<'a, Slow>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut equip_dirty,
            entities,
            items,
            backpacks,
            wielded,
            mut pools,
            mut attributes,
            player,
            mut gamelog,
            attr_bonus,
            statuses,
            slows,
        ) = data;

        if equip_dirty.is_empty() {
            return;
        }

        #[derive(Default, Debug)]
        struct ItemUpdate {
            weight: f32,
            initiative: f32,
            might: i32,
            fitness: i32,
            quickness: i32,
            intelligence: i32,
        }

        let mut to_update = HashMap::new();
        for (entity, _dirty) in (&entities, &equip_dirty).join() {
            to_update.insert(entity, ItemUpdate::default());
        }

        equip_dirty.clear();

        for (item, equipped, entity) in (&items, &wielded, &entities).join() {
            if to_update.contains_key(&equipped.owner) {
                let totals = to_update.get_mut(&equipped.owner).unwrap();
                totals.weight += item.weight;
                totals.initiative += item.initiative_penalty;
                if let Some(attr) = attr_bonus.get(entity) {
                    totals.might += attr.might.unwrap_or(0);
                    totals.fitness += attr.fitness.unwrap_or(0);
                    totals.quickness += attr.quickness.unwrap_or(0);
                    totals.intelligence += attr.intelligence.unwrap_or(0);
                }
            }
        }

        for (item, carried) in (&items, &backpacks).join() {
            if to_update.contains_key(&carried.owner) {
                let totals = to_update.get_mut(&carried.owner).unwrap();
                totals.weight += item.weight;
                totals.initiative += item.initiative_penalty;
            }
        }

        for (status, attr) in (&statuses, &attr_bonus).join() {
            if to_update.contains_key(&status.target) {
                let totals = to_update.get_mut(&status.target).unwrap();
                totals.might += attr.might.unwrap_or(0);
                totals.fitness += attr.fitness.unwrap_or(0);
                totals.quickness += attr.quickness.unwrap_or(0);
                totals.intelligence += attr.intelligence.unwrap_or(0);
            }
        }

        for (status, slow) in (&statuses, &slows).join() {
            if to_update.contains_key(&status.target) {
                let totals = to_update.get_mut(&status.target).unwrap();
                totals.initiative += slow.initiative_penalty;
            }
        }

        for (entity, item_update) in &to_update {
            if let Some(pool) = pools.get_mut(*entity) {
                pool.total_weight = item_update.weight;
                pool.total_initiative_penalty = item_update.initiative;

                if let Some(attr) = attributes.get_mut(*entity) {
                    attr.might.modifiers = item_update.might;
                    attr.fitness.modifiers = item_update.fitness;
                    attr.quickness.modifiers = item_update.quickness;
                    attr.intelligence.modifiers = item_update.intelligence;

                    attr.might.bonus = item_update.might;
                    attr.fitness.bonus = item_update.fitness;
                    attr.quickness.bonus = item_update.quickness;
                    attr.intelligence.bonus = item_update.intelligence;

                    let carry_capacity = attr.get_max_carry_capacity();
                    if pool.total_weight as u32 > carry_capacity {
                        pool.total_initiative_penalty += 4.0;
                        if *entity == *player {
                            gamelog.entries.push(
                                "You are overburdened, and suffering an initiative penalty.".into(),
                            );
                        }
                    }
                }
            }
        }
    }
}
