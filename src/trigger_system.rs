use specs::{Entities, Entity, Join, ReadExpect, ReadStorage, System, WriteExpect, WriteStorage};

use crate::components::{
    ApplyTeleport, EntityMoved, EntryTrigger, Hidden, InflictsDamage, Name, Position,
    SingleActivation, SufferDamage, TeleportTo,
};
use crate::gamelog::GameLog;
use crate::map::Map;
use crate::particle_system::ParticleBuilder;
use crate::spatial;

pub struct TriggerSystem {}

impl<'a> System<'a> for TriggerSystem {
    type SystemData = (
        ReadExpect<'a, Map>,
        WriteStorage<'a, EntityMoved>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, EntryTrigger>,
        WriteStorage<'a, Hidden>,
        ReadStorage<'a, Name>,
        Entities<'a>,
        WriteExpect<'a, GameLog>,
        ReadStorage<'a, InflictsDamage>,
        WriteStorage<'a, SufferDamage>,
        WriteExpect<'a, ParticleBuilder>,
        ReadStorage<'a, SingleActivation>,
        ReadStorage<'a, TeleportTo>,
        WriteStorage<'a, ApplyTeleport>,
        ReadExpect<'a, Entity>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            map,
            mut entity_moved,
            position,
            entry_trigger,
            mut hidden,
            name,
            entities,
            mut log,
            inflicts_damage,
            mut inflict_damage,
            mut particle_builder,
            single_activation,
            teleporters,
            mut apply_teleport,
            player_entity,
        ) = data;

        let mut remove_entity = Vec::new();

        for (entity, mut _entity_moved, pos) in (&entities, &mut entity_moved, &position).join() {
            let idx = map.xy_idx(pos.x, pos.y);
            spatial::for_each_tile_content(idx, |entity_id| {
                if entity == entity_id {
                    return;
                }

                let maybe_trigger = entry_trigger.get(entity_id);
                match maybe_trigger {
                    None => {}
                    Some(_trigger) => {
                        let name = name.get(entity_id);
                        if let Some(name) = name {
                            log.entries.push(format!("{} triggers!", &name.name));
                        }

                        hidden.remove(entity_id);

                        let damage = inflicts_damage.get(entity_id);
                        if let Some(damage) = damage {
                            particle_builder.request(
                                pos.x,
                                pos.y,
                                rltk::RGB::named(rltk::ORANGE),
                                rltk::RGB::named(rltk::BLACK),
                                rltk::to_cp437('‼'),
                                200.0,
                            );
                            SufferDamage::new_damage(
                                &mut inflict_damage,
                                entity,
                                damage.damage,
                                false,
                            );
                        }

                        if let Some(teleport) = teleporters.get(entity_id) {
                            if (teleport.player_only && entity == *player_entity)
                                || !teleport.player_only
                            {
                                apply_teleport
                                    .insert(
                                        entity,
                                        ApplyTeleport {
                                            dest_x: teleport.x,
                                            dest_y: teleport.y,
                                            dest_depth: teleport.depth,
                                        },
                                    )
                                    .expect("Unable to insert");
                            }
                        }

                        let sa = single_activation.get(entity_id);
                        if let Some(_sa) = sa {
                            remove_entity.push(entity_id);
                        }
                    }
                }
            });
        }

        for trap in remove_entity.iter() {
            entities.delete(*trap).expect("Unable to delete trap");
        }

        entity_moved.clear();
    }
}
