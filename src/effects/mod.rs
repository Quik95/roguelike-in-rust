use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;

use lazy_static::lazy_static;
use rltk::{FontCharType, Point, RGB};
use specs::{Entity, World};

pub use targeting::*;

use crate::components::AttributeBonus;

mod damage;
mod hunger;
mod movement;
mod particles;
mod targeting;
mod triggers;

lazy_static! {
    pub static ref EFFECT_QUEUE: Mutex<VecDeque<EffectSpawner>> = Mutex::new(VecDeque::new());
}

pub enum EffectType {
    Bloodstain,
    EntityDeath,
    WellFed,
    Damage {
        amount: i32,
    },
    Healing {
        amount: i32,
    },
    Mana {
        amount: i32,
    },
    Confusion {
        turns: i32,
    },
    ItemUse {
        item: Entity,
    },
    TriggerFire {
        trigger: Entity,
    },
    SpellUse {
        spell: Entity,
    },
    Slow {
        inititive_penalty: f32,
    },
    DamageOverTime {
        damage: i32,
    },
    TeleportTo {
        x: i32,
        y: i32,
        depth: i32,
        player_only: bool,
    },
    AttributeEffect {
        bonus: AttributeBonus,
        name: String,
        duration: i32,
    },
    Particle {
        glyph: FontCharType,
        fg: RGB,
        bg: RGB,
        lifespan: f32,
    },
    ParticleProjectile {
        glyph: FontCharType,
        fg: RGB,
        bg: RGB,
        lifespan: f32,
        speed: f32,
        path: Vec<Point>,
    },
}

#[derive(Clone)]
pub enum Targets {
    Single { target: Entity },
    Tile { tile_idx: i32 },
    Tiles { tiles: Vec<i32> },
}

pub struct EffectSpawner {
    pub creator: Option<Entity>,
    pub effect_type: EffectType,
    pub targets: Targets,
    dedupe: HashSet<Entity>,
}

pub fn add_effect(creator: Option<Entity>, effect_type: EffectType, targets: Targets) {
    EFFECT_QUEUE.lock().unwrap().push_back(EffectSpawner {
        creator,
        effect_type,
        targets,
        dedupe: HashSet::new(),
    });
}

pub fn run_effects_queue(ecs: &mut World) {
    loop {
        let effect = EFFECT_QUEUE.lock().unwrap().pop_front();
        if let Some(mut effect) = effect {
            target_applicator(ecs, &mut effect);
        } else {
            break;
        }
    }
}

fn target_applicator(ecs: &mut World, effect: &mut EffectSpawner) {
    if let EffectType::ItemUse { item } = effect.effect_type {
        triggers::item_trigger(effect.creator, item, &effect.targets, ecs);
    } else if let EffectType::SpellUse { spell } = effect.effect_type {
        triggers::spell_trigger(effect.creator, spell, &effect.targets, ecs);
    } else if let EffectType::TriggerFire { trigger } = effect.effect_type {
        triggers::trigger(effect.creator, trigger, &effect.targets, ecs);
    } else {
        match &effect.targets.clone() {
            Targets::Single { target } => affect_entity(ecs, effect, *target),
            Targets::Tile { tile_idx } => affect_tile(ecs, effect, *tile_idx),
            Targets::Tiles { tiles } => tiles
                .iter()
                .for_each(|tile_idx| affect_tile(ecs, effect, *tile_idx)),
        }
    }
}

fn affect_entity(ecs: &mut World, effect: &mut EffectSpawner, target: Entity) {
    if effect.dedupe.contains(&target) {
        return;
    }

    effect.dedupe.insert(target);
    match &effect.effect_type {
        EffectType::Damage { .. } => damage::inflict_damage(ecs, effect, target),
        EffectType::Bloodstain { .. } => {
            if let Some(pos) = entity_position(ecs, target) {
                damage::bloodstain(ecs, pos);
            }
        }
        EffectType::Particle { .. } => {
            if let Some(pos) = entity_position(ecs, target) {
                particles::particle_to_tile(ecs, pos, effect);
            }
        }
        EffectType::EntityDeath => damage::death(ecs, effect, target),
        EffectType::WellFed => hunger::well_fed(ecs, effect, target),
        EffectType::Healing { .. } => damage::heal_damage(ecs, effect, target),
        EffectType::Confusion { .. } => damage::add_confusion(ecs, effect, target),
        EffectType::TeleportTo { .. } => movement::apply_teleport(ecs, effect, target),
        EffectType::AttributeEffect { .. } => damage::attribute_effect(ecs, effect, target),
        EffectType::Mana { .. } => damage::restore_mana(ecs, effect, target),
        EffectType::Slow { .. } => damage::slow(ecs, effect, target),
        EffectType::DamageOverTime { .. } => damage::damage_over_time(ecs, effect, target),
        _ => {}
    }
}

fn affect_tile(ecs: &mut World, effect: &mut EffectSpawner, tile_idx: i32) {
    if tile_effect_hits_entities(&effect.effect_type) {
        let content = crate::spatial::get_tile_content_clone(tile_idx as usize);
        content
            .iter()
            .for_each(|entity| affect_entity(ecs, effect, *entity));
    }

    match &effect.effect_type {
        EffectType::Bloodstain => damage::bloodstain(ecs, tile_idx),
        EffectType::Particle { .. } => particles::particle_to_tile(ecs, tile_idx, effect),
        EffectType::ParticleProjectile { .. } => particles::projectile(ecs, tile_idx, &effect),
        _ => {}
    }
}

const fn tile_effect_hits_entities(effect: &EffectType) -> bool {
    matches!(
        effect,
        EffectType::Damage { .. }
            | EffectType::WellFed
            | EffectType::Mana { .. }
            | EffectType::Healing { .. }
            | EffectType::Confusion { .. }
            | EffectType::TeleportTo { .. }
            | EffectType::AttributeEffect { .. }
            | EffectType::Slow { .. }
            | EffectType::DamageOverTime { .. }
    )
}
