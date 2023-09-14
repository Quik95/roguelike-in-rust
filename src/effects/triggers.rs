use rltk::{Point, BLACK};
use specs::{Entity, World, WorldExt};

use crate::components::{
    AlwaysTargetsSelf, AreaOfEffect, AttributeBonus, Confusion, Consumable, DamageOverTime,
    Duration, Hidden, InflictsDamage, KnownSpell, KnownSpells, MagicMapper, Name, Pools, Position,
    ProvidesFood, ProvidesHealing, ProvidesIdentification, ProvidesMana, ProvidesRemoveCurse,
    SingleActivation, Slow, SpawnParticleBurst, SpawnParticleLine, SpellTemplate, TeachesSpell,
    TeleportTo, TownPortal,
};
use crate::effects::targeting::entity_position;

use crate::effects::{add_effect, aoe_tiles, find_item_position, EffectType, Targets};
use crate::gamelog::GameLog;
use crate::map::Map;
use crate::player::RunState;
use crate::raws::rawmaster::find_spell_entity;

pub fn item_trigger(creator: Option<Entity>, item: Entity, targets: &Targets, ecs: &World) {
    if let Some(c) = ecs.write_storage::<Consumable>().get_mut(item) {
        if c.charges < 1 {
            let mut gamelog = ecs.fetch_mut::<GameLog>();
            gamelog.entries.push(format!(
                "{} is out of charges!",
                ecs.read_storage::<Name>().get(item).unwrap().name
            ));
            return;
        } else {
            c.charges -= 1;
        }
    }

    let did_something = event_trigger(creator, item, targets, ecs);

    if did_something {
        if let Some(c) = ecs.read_storage::<Consumable>().get(item) {
            if c.charges == 0 {
                ecs.entities().delete(item).expect("Delete failed");
            }
        }
    }
}

fn event_trigger(creator: Option<Entity>, entity: Entity, targets: &Targets, ecs: &World) -> bool {
    let mut did_something = false;
    let mut gamelog = ecs.fetch_mut::<GameLog>();

    if let Some(part) = ecs.read_storage::<SpawnParticleBurst>().get(entity) {
        add_effect(
            creator,
            EffectType::Particle {
                glyph: part.glyph,
                fg: part.color,
                bg: BLACK.into(),
                lifespan: part.lifetime_ms,
            },
            targets.clone(),
        );
    }

    if let Some(part) = ecs.read_storage::<SpawnParticleLine>().get(entity) {
        if let Some(start_pos) = find_item_position(ecs, entity, creator) {
            match targets {
                Targets::Tile { tile_idx } => spawn_line_particles(ecs, start_pos, *tile_idx, part),
                Targets::Tiles { tiles } => tiles
                    .iter()
                    .for_each(|tile_idx| spawn_line_particles(ecs, start_pos, *tile_idx, part)),
                Targets::Single { target } => {
                    if let Some(end_pos) = entity_position(ecs, *target) {
                        spawn_line_particles(ecs, start_pos, end_pos, part);
                    }
                }
            }
        }
    }

    if ecs.read_storage::<ProvidesFood>().get(entity).is_some() {
        add_effect(creator, EffectType::WellFed, targets.clone());
        let names = ecs.read_storage::<Name>();
        gamelog
            .entries
            .push(format!("You eat the {}.", names.get(entity).unwrap().name));
        did_something = true;
    }

    if ecs.read_storage::<MagicMapper>().get(entity).is_some() {
        let mut runstate = ecs.fetch_mut::<RunState>();
        gamelog.entries.push("The map is revealed to you!".into());
        *runstate = RunState::MagicMapReveal { row: 0 };
        did_something = true;
    }

    if ecs
        .read_storage::<ProvidesRemoveCurse>()
        .get(entity)
        .is_some()
    {
        let mut runstate = ecs.fetch_mut::<RunState>();
        *runstate = RunState::ShowRemoveCurse;
        did_something = true;
    }

    if ecs
        .read_storage::<ProvidesIdentification>()
        .get(entity)
        .is_some()
    {
        let mut runstate = ecs.fetch_mut::<RunState>();
        *runstate = RunState::ShowIdentify;
        did_something = true;
    }

    if ecs.read_storage::<TownPortal>().get(entity).is_some() {
        let map = ecs.fetch::<Map>();
        if map.depth == 1 {
            gamelog
                .entries
                .push("You are already in town, so the scroll does nothing.".into());
        } else {
            gamelog
                .entries
                .push("You are teleported back to town!".into());
            let mut runstate = ecs.fetch_mut::<RunState>();
            *runstate = RunState::TownPortal;
            did_something = true;
        }
    }

    if let Some(heal) = ecs.read_storage::<ProvidesHealing>().get(entity) {
        add_effect(
            creator,
            EffectType::Healing {
                amount: heal.heal_amount,
            },
            targets.clone(),
        );
        did_something = true;
    }

    if let Some(mana) = ecs.read_storage::<ProvidesMana>().get(entity) {
        add_effect(
            creator,
            EffectType::Mana {
                amount: mana.mana_amount,
            },
            targets.clone(),
        );
        did_something = true;
    }

    if let Some(damage) = ecs.read_storage::<InflictsDamage>().get(entity) {
        add_effect(
            creator,
            EffectType::Damage {
                amount: damage.damage,
            },
            targets.clone(),
        );
        did_something = true;
    }

    if let Some(_confusion) = ecs.read_storage::<Confusion>().get(entity) {
        if let Some(duration) = ecs.read_storage::<Duration>().get(entity) {
            add_effect(
                creator,
                EffectType::Confusion {
                    turns: duration.turns,
                },
                targets.clone(),
            );
            did_something = true;
        }
    }

    if let Some(teleport) = ecs.read_storage::<TeleportTo>().get(entity) {
        add_effect(
            creator,
            EffectType::TeleportTo {
                x: teleport.x,
                y: teleport.y,
                depth: teleport.depth,
                player_only: teleport.player_only,
            },
            targets.clone(),
        );
        did_something = true;
    }

    if let Some(attr) = ecs.read_storage::<AttributeBonus>().get(entity) {
        add_effect(
            creator,
            EffectType::AttributeEffect {
                bonus: attr.clone(),
                name: ecs.read_storage::<Name>().get(entity).unwrap().name.clone(),
                duration: 10,
            },
            targets.clone(),
        );
        did_something = true;
    }

    if let Some(spell) = ecs.read_storage::<TeachesSpell>().get(entity) {
        if let Some(known) = ecs.write_storage::<KnownSpells>().get_mut(creator.unwrap()) {
            if let Some(spell_entity) = find_spell_entity(ecs, &spell.spell) {
                if let Some(spell_info) = ecs.read_storage::<SpellTemplate>().get(spell_entity) {
                    let mut already_known = false;
                    known.spells.iter().for_each(|s| {
                        if s.display_name == spell.spell {
                            already_known = true;
                        }
                    });
                    if !already_known {
                        known.spells.push(KnownSpell {
                            display_name: spell.spell.clone(),
                            mana_cost: spell_info.mana_cost,
                        });
                    }
                }
            }
        }
        did_something = true;
    }

    if let Some(slow) = ecs.read_storage::<Slow>().get(entity) {
        add_effect(
            creator,
            EffectType::Slow {
                inititive_penalty: slow.initiative_penalty,
            },
            targets.clone(),
        );
        did_something = true;
    }

    if let Some(damage) = ecs.read_storage::<DamageOverTime>().get(entity) {
        add_effect(
            creator,
            EffectType::DamageOverTime {
                damage: damage.damage,
            },
            targets.clone(),
        );
        did_something = true;
    }

    did_something
}

pub fn trigger(creator: Option<Entity>, trigger: Entity, targets: &Targets, ecs: &World) {
    ecs.write_storage::<Hidden>().remove(trigger);

    let did_something = event_trigger(creator, trigger, targets, ecs);

    if did_something
        && ecs
            .read_storage::<SingleActivation>()
            .get(trigger)
            .is_some()
    {
        ecs.entities().delete(trigger).expect("Delete Failed");
    }
}

fn spawn_line_particles(ecs: &World, start: i32, end: i32, part: &SpawnParticleLine) {
    let map = ecs.fetch::<Map>();
    let start_pt = Point::new(start % map.width, end / map.width);
    let end_pt = Point::new(end % map.width, end / map.width);
    let line = rltk::line2d_bresenham(start_pt, end_pt);
    for pt in &line {
        add_effect(
            None,
            EffectType::Particle {
                glyph: part.glyph,
                fg: part.color,
                bg: BLACK.into(),
                lifespan: part.lifetime_ms,
            },
            Targets::Tile {
                tile_idx: map.xy_idx(pt.x, pt.y) as i32,
            },
        );
    }
}

pub fn spell_trigger(creator: Option<Entity>, spell: Entity, targets: &Targets, ecs: &World) {
    let mut self_destruct = false;
    let mut targeting = targets.clone();
    if let Some(template) = ecs.read_storage::<SpellTemplate>().get(spell) {
        let mut pools = ecs.write_storage::<Pools>();
        if let Some(caster) = creator {
            if let Some(pool) = pools.get_mut(caster) {
                if template.mana_cost <= pool.mana.current {
                    pool.mana.current -= template.mana_cost;
                }
            }

            if ecs.read_storage::<AlwaysTargetsSelf>().get(spell).is_some() {
                if let Some(pos) = ecs.read_storage::<Position>().get(caster) {
                    let map = ecs.fetch::<Map>();
                    targeting = if let Some(aoe) = ecs.read_storage::<AreaOfEffect>().get(spell) {
                        Targets::Tiles {
                            tiles: aoe_tiles(&map, Point::new(pos.x, pos.y), aoe.radius),
                        }
                    } else {
                        Targets::Tile {
                            tile_idx: map.xy_idx(pos.x, pos.y) as i32,
                        }
                    }
                }
            }
        }
        if let Some(_destruct) = ecs.read_storage::<SingleActivation>().get(spell) {
            self_destruct = true;
        }
    }

    event_trigger(creator, spell, &targeting, ecs);
    if !self_destruct {
        return;
    }

    if let Some(creator) = creator {
        ecs.entities()
            .delete(creator)
            .expect("Unable to delete owner");
    }
}
