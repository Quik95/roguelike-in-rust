use std::collections::HashMap;

use specs::{Entities, Entity, Join, ReadExpect, ReadStorage, System, WriteExpect, WriteStorage};

use crate::components::{Attributes, EquipmentChanged, Equipped, InBackpack, Item, Pools};
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
        ReadStorage<'a, Attributes>,
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut equip_dirty,
            entities,
            items,
            backpacks,
            wielded,
            mut pools,
            attributes,
            player,
            mut gamelog,
        ) = data;

        if equip_dirty.is_empty() {
            return;
        }

        let mut to_update = HashMap::new();
        for (entity, _dirty) in (&entities, &equip_dirty).join() {
            to_update.insert(entity, (0.0, 0.0));
        }

        equip_dirty.clear();

        for (item, equipped) in (&items, &wielded).join() {
            if to_update.contains_key(&equipped.owner) {
                let totals = to_update.get_mut(&equipped.owner).unwrap();
                totals.0 += item.weight;
                totals.1 += item.initiative_penalty;
            }
        }

        for (item, carried) in (&items, &backpacks).join() {
            if to_update.contains_key(&carried.owner) {
                let totals = to_update.get_mut(&carried.owner).unwrap();
                totals.0 += item.weight;
                totals.1 += item.initiative_penalty;
            }
        }

        for (entity, (weight, initiative)) in &to_update {
            if let Some(pool) = pools.get_mut(*entity) {
                pool.total_weight = *weight;
                pool.total_initiative_penalty = *initiative;

                if let Some(attr) = attributes.get(*entity) {
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
