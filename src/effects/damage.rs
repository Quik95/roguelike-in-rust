use rltk::{to_cp437, Point, BLACK, BLUE, GOLD, GREEN, ORANGE};
use specs::saveload::{MarkedBuilder, SimpleMarker};
use specs::{Builder, Entity, World, WorldExt};

use crate::components::{
    Attributes, Confusion, DamageOverTime, Duration, EquipmentChanged, Name, Player, Pools,
    SerializeMe, Slow, StatusEffect,
};
use crate::effects::targeting::entity_position;
use crate::effects::{add_effect, EffectSpawner, EffectType, Targets};
use crate::gamelog::GameLog;
use crate::gamesystem::{mana_at_level, player_hp_at_level};
use crate::map::Map;

pub fn inflict_damage(ecs: &World, damage: &EffectSpawner, target: Entity) {
    let mut pools = ecs.write_storage::<Pools>();
    if let Some(pool) = pools.get_mut(target) {
        if pool.god_mode {
            return;
        }

        if let EffectType::Damage { amount } = damage.effect_type {
            pool.hit_points.current -= amount;

            add_effect(None, EffectType::Bloodstain, Targets::Single { target });
            add_effect(
                None,
                EffectType::Particle {
                    glyph: to_cp437('‼'),
                    fg: ORANGE.into(),
                    bg: BLACK.into(),
                    lifespan: 200.0,
                },
                Targets::Single { target },
            );

            if pool.hit_points.current < 1 {
                add_effect(
                    damage.creator,
                    EffectType::EntityDeath,
                    Targets::Single { target },
                );
            }
        }
    }
}

pub fn bloodstain(ecs: &World, tile_idx: i32) {
    let mut map = ecs.fetch_mut::<Map>();
    map.bloodstains.insert(tile_idx as usize);
}

pub fn death(ecs: &World, effect: &EffectSpawner, target: Entity) {
    let mut xp_gain = 0;
    let mut gold_gain = 0.0_f32;

    let mut pools = ecs.write_storage::<Pools>();
    let attributes = ecs.read_storage::<Attributes>();

    if let Some(pos) = entity_position(ecs, target) {
        crate::spatial::remove_entity(target, pos as usize);
    }

    if let Some(source) = effect.creator {
        if ecs.read_storage::<Player>().get(source).is_some() {
            if let Some(stats) = pools.get(target) {
                xp_gain += stats.level * 100;
                gold_gain += stats.gold;
            }

            if xp_gain != 0 || gold_gain != 0.0 {
                let mut log = ecs.fetch_mut::<GameLog>();
                let player_stats = pools.get_mut(source).unwrap();
                let player_attributes = attributes.get(source).unwrap();
                player_stats.xp += xp_gain;
                player_stats.gold += gold_gain;
                if player_stats.xp >= player_stats.level * 1000 {
                    // We've gone up a level!
                    player_stats.level += 1;
                    log.entries.push(format!(
                        "Congratulations, you are now level {}",
                        player_stats.level
                    ));
                    player_stats.hit_points.max = player_hp_at_level(
                        player_attributes.fitness.base + player_attributes.fitness.modifiers,
                        player_stats.level,
                    );
                    player_stats.hit_points.current = player_stats.hit_points.max;
                    player_stats.mana.max = mana_at_level(
                        player_attributes.intelligence.base
                            + player_attributes.intelligence.modifiers,
                        player_stats.level,
                    );
                    player_stats.mana.current = player_stats.mana.max;

                    let player_pos = ecs.fetch::<Point>();
                    let map = ecs.fetch_mut::<Map>();
                    for i in 0..10 {
                        if player_pos.y - i > 1 {
                            add_effect(
                                None,
                                EffectType::Particle {
                                    glyph: to_cp437('░'),
                                    fg: GOLD.into(),
                                    bg: BLACK.into(),
                                    lifespan: 400.0,
                                },
                                Targets::Tile {
                                    tile_idx: map.xy_idx(player_pos.x, player_pos.y - i) as i32,
                                },
                            );
                        }
                    }
                }
            }
        }
    }
}

pub fn heal_damage(ecs: &World, heal: &EffectSpawner, target: Entity) {
    let mut pools = ecs.write_storage::<Pools>();
    if let Some(pool) = pools.get_mut(target) {
        if let EffectType::Healing { amount } = heal.effect_type {
            pool.hit_points.current =
                i32::min(pool.hit_points.max, pool.hit_points.current + amount);
            add_effect(
                None,
                EffectType::Particle {
                    glyph: to_cp437('‼'),
                    fg: GREEN.into(),
                    bg: BLACK.into(),
                    lifespan: 0.0,
                },
                Targets::Single { target },
            );
        }
    }
}

pub fn add_confusion(ecs: &mut World, effect: &EffectSpawner, target: Entity) {
    if let EffectType::Confusion { turns } = &effect.effect_type {
        ecs.create_entity()
            .with(StatusEffect { target })
            .with(Confusion {})
            .with(Duration { turns: *turns })
            .with(Name {
                name: "Confusion".into(),
            })
            .marked::<SimpleMarker<SerializeMe>>()
            .build();
    }
}

pub fn attribute_effect(ecs: &mut World, effect: &EffectSpawner, target: Entity) {
    if let EffectType::AttributeEffect {
        bonus,
        name,
        duration,
    } = &effect.effect_type
    {
        ecs.create_entity()
            .with(StatusEffect { target })
            .with(bonus.clone())
            .with(Duration { turns: *duration })
            .with(Name { name: name.clone() })
            .marked::<SimpleMarker<SerializeMe>>()
            .build();

        ecs.write_storage::<EquipmentChanged>()
            .insert(target, EquipmentChanged {})
            .expect("Insert failed");
    }
}

pub(crate) fn restore_mana(ecs: &mut World, mana: &EffectSpawner, target: Entity) {
    let mut pools = ecs.write_storage::<Pools>();
    if let Some(pool) = pools.get_mut(target) {
        if let EffectType::Mana { amount } = mana.effect_type {
            pool.mana.current = i32::min(pool.mana.max, pool.mana.current + amount);
            add_effect(
                None,
                EffectType::Particle {
                    glyph: to_cp437('‼'),
                    fg: BLUE.into(),
                    bg: BLACK.into(),
                    lifespan: 200.0,
                },
                Targets::Single { target },
            );
        }
    }
}

pub(crate) fn slow(ecs: &mut World, effect: &EffectSpawner, target: Entity) {
    if let EffectType::Slow { inititive_penalty } = &effect.effect_type {
        ecs.create_entity()
            .with(StatusEffect { target })
            .with(Slow {
                initiative_penalty: *inititive_penalty,
            })
            .with(Duration { turns: 5 })
            .with(if *inititive_penalty > 0.0 {
                Name {
                    name: "Slowed".into(),
                }
            } else {
                Name {
                    name: "Hasted".into(),
                }
            })
            .marked::<SimpleMarker<SerializeMe>>()
            .build();
    }
}

pub(crate) fn damage_over_time(ecs: &mut World, effect: &EffectSpawner, target: Entity) {
    if let EffectType::DamageOverTime { damage } = &effect.effect_type {
        ecs.create_entity()
            .with(StatusEffect { target })
            .with(DamageOverTime { damage: *damage })
            .with(Duration { turns: 5 })
            .with(Name {
                name: "Damage Over Time".into(),
            })
            .marked::<SimpleMarker<SerializeMe>>()
            .build();
    }
}
