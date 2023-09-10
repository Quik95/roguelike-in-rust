use rltk::{Point, BLACK};
use specs::{Entity, World, WorldExt};

use crate::components::{
    AttributeBonus, Confusion, Consumable, Duration, Hidden, InflictsDamage, MagicMapper, Name,
    ProvidesFood, ProvidesHealing, ProvidesIdentification, ProvidesRemoveCurse, SingleActivation,
    SpawnParticleBurst, SpawnParticleLine, TeleportTo, TownPortal,
};
use crate::effects::targeting::entity_position;
use crate::effects::{add_effect, find_item_position, EffectType, Targets};
use crate::gamelog::GameLog;
use crate::map::Map;
use crate::player::RunState;

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
        if let Some(start_pos) = find_item_position(ecs, entity) {
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
