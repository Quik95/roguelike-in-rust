use specs::{Entities, Entity, Join, ReadExpect, ReadStorage, System, WriteExpect, WriteStorage};

use crate::components::{AreaOfEffect, EquipmentChanged, IdentifiedItem, Name, WantsToCastSpell};
use crate::effects::{add_effect, aoe_tiles, EffectType, Targets};
use crate::map::Map;

pub struct SpellUseSystem {}

impl<'a> System<'a> for SpellUseSystem {
    type SystemData = (
        ReadExpect<'a, Entity>,
        WriteExpect<'a, Map>,
        Entities<'a>,
        WriteStorage<'a, WantsToCastSpell>,
        ReadStorage<'a, Name>,
        ReadStorage<'a, AreaOfEffect>,
        WriteStorage<'a, EquipmentChanged>,
        WriteStorage<'a, IdentifiedItem>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            player_entity,
            map,
            entities,
            mut wants_use,
            names,
            aoe,
            mut dirty,
            mut identified_item,
        ) = data;

        for (entity, useitem) in (&entities, &wants_use).join() {
            dirty
                .insert(entity, EquipmentChanged {})
                .expect("Unable to insert");

            if entity == *player_entity {
                identified_item
                    .insert(
                        entity,
                        IdentifiedItem {
                            name: names.get(useitem.spell).unwrap().name.clone(),
                        },
                    )
                    .expect("Unable to insert");
            }

            add_effect(
                Some(entity),
                EffectType::SpellUse {
                    spell: useitem.spell,
                },
                match useitem.target {
                    None => Targets::Single {
                        target: *player_entity,
                    },
                    Some(target) => {
                        if let Some(aoe) = aoe.get(useitem.spell) {
                            Targets::Tiles {
                                tiles: aoe_tiles(&*map, target, aoe.radius),
                            }
                        } else {
                            Targets::Tile {
                                tile_idx: map.xy_idx(target.x, target.y) as i32,
                            }
                        }
                    }
                },
            );
        }

        wants_use.clear();
    }
}
