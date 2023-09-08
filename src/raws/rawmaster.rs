use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use lazy_static::lazy_static;
use rltk::{console, to_cp437, RandomNumberGenerator, RGB};
use specs::saveload::{MarkedBuilder, SimpleMarker};
use specs::{Builder, Entity, EntityBuilder, World, WorldExt};

use crate::components::{
    AreaOfEffect, Attribute, Attributes, BlocksTile, BlocksVisibility, Confusion, Consumable, Door,
    EntryTrigger, EquipmentChanged, EquipmentSlot, Equippable, Faction, Hidden, InBackpack,
    InflictsDamage, Initiative, LightSource, MagicMapper, MeleeWeapon, MoveMode, Movement, Name,
    NaturalAttack, NaturalAttackDefense, Pool, Pools, Position, ProvidesFood, ProvidesHealing,
    Ranged, SerializeMe, SingleActivation, Skill, Skills, Vendor, Viewshed, WeaponAttribute,
    Wearable,
};
use crate::components::{Equipped, LootTable};
use crate::components::{Quips, Renderable};
use crate::gamesystem::{attr_bonus, mana_at_level, npc_hp, DiceRoll};
use crate::random_table::RandomTable;
use crate::raws::faction_structs::Reaction;
use crate::raws::spawn_table_structs::SpawnTableEntry;
use crate::raws::Raws;

lazy_static! {
    pub static ref RAWS: Mutex<RawMaster> = Mutex::new(RawMaster::default());
}

pub const LBS_TO_KG_RATIO: f32 = 2.205;

#[derive(Eq, PartialEq, Hash, Copy, Clone)]
pub enum SpawnType {
    AtPosition { x: i32, y: i32 },
    Equipped { by: Entity },
    Carried { by: Entity },
}

#[derive(Default)]
pub struct RawMaster {
    raws: Raws,
    item_index: HashMap<String, usize>,
    mob_index: HashMap<String, usize>,
    prop_index: HashMap<String, usize>,
    loot_index: HashMap<String, usize>,
    faction_index: HashMap<String, HashMap<String, Reaction>>,
}

impl RawMaster {
    pub fn load(&mut self, raws: Raws) {
        self.raws = raws;
        self.item_index = HashMap::new();
        let mut used_names: HashSet<String> = HashSet::new();
        for (i, item) in self.raws.items.iter().enumerate() {
            if used_names.contains(&item.name) {
                console::log(format!(
                    "WARNING - duplicate item name in raws [{}]",
                    item.name
                ));
            }
            self.item_index.insert(item.name.clone(), i);
            used_names.insert(item.name.clone());
        }
        for (i, mob) in self.raws.mobs.iter().enumerate() {
            if used_names.contains(&mob.name) {
                console::log(format!(
                    "WARNING - duplicate mob name in raws [{}]",
                    mob.name
                ));
            }
            self.mob_index.insert(mob.name.clone(), i);
            used_names.insert(mob.name.clone());
        }

        for (i, prop) in self.raws.props.iter().enumerate() {
            if used_names.contains(&prop.name) {
                console::log(format!(
                    "WARNING - duplicate prop name in raws [{}]",
                    prop.name
                ));
            }
            self.prop_index.insert(prop.name.clone(), i);
            used_names.insert(prop.name.clone());
        }

        for spawn in self.raws.spawn_table.iter() {
            if !used_names.contains(&spawn.name) {
                console::log(format!(
                    "WARNING - Spawn table references unspecified entity {}",
                    spawn.name
                ));
            }
        }

        for (i, loot) in self.raws.loot_tables.iter().enumerate() {
            self.loot_index.insert(loot.name.clone(), i);
        }

        for faction in self.raws.faction_table.iter() {
            let mut reactions = HashMap::new();
            for other in faction.responses.iter() {
                reactions.insert(
                    other.0.clone(),
                    match other.1.as_str() {
                        "ignore" => Reaction::Ignore,
                        "flee" => Reaction::Flee,
                        _ => Reaction::Attack,
                    },
                );
            }

            self.faction_index.insert(faction.name.clone(), reactions);
        }
    }
}

fn spawn_position<'a>(
    pos: SpawnType,
    new_entity: EntityBuilder<'a>,
    tag: &str,
    raws: &RawMaster,
) -> EntityBuilder<'a> {
    let eb = new_entity;

    match pos {
        SpawnType::AtPosition { x, y } => eb.with(Position { x, y }),
        SpawnType::Carried { by } => eb.with(InBackpack { owner: by }),
        SpawnType::Equipped { by } => {
            let slot = find_slot_for_equippable_item(tag, raws);
            eb.with(Equipped { owner: by, slot })
        }
    }
}

fn get_renderable_component(renderable: &super::item_structs::Renderable) -> Renderable {
    Renderable {
        glyph: to_cp437(renderable.glyph.chars().next().unwrap()),
        fg: RGB::from_hex(&renderable.fg).expect("Invalid RGB"),
        bg: RGB::from_hex(&renderable.bg).expect("Invalid RGB"),
        render_order: renderable.order,
    }
}

pub fn spawn_named_item(
    raws: &RawMaster,
    ecs: &mut World,
    key: &str,
    pos: SpawnType,
) -> Option<Entity> {
    if raws.item_index.contains_key(key) {
        let item_template = &raws.raws.items[raws.item_index[key]];

        let mut eb = ecs.create_entity().marked::<SimpleMarker<SerializeMe>>();

        eb = spawn_position(pos, eb, key, raws);

        if let Some(renderable) = &item_template.renderable {
            eb = eb.with(get_renderable_component(renderable));
        }

        eb = eb.with(Name {
            name: item_template.name.clone(),
        });
        eb = eb.with(crate::components::Item {
            initiative_penalty: item_template.initiative_penalty.unwrap_or(0.0),
            weight: item_template
                .weight_lbs
                .map_or_else(|| 0.0, |lbs| lbs / LBS_TO_KG_RATIO),
            base_value: item_template.base_value.unwrap_or(0.0),
        });

        if let Some(consumable) = &item_template.consumable {
            eb = eb.with(Consumable {});
            for effect in consumable.effects.iter() {
                let effect_name = effect.0.as_str();
                match effect_name {
                    "provides_healing" => {
                        eb = eb.with(ProvidesHealing {
                            heal_amount: effect.1.parse::<i32>().unwrap(),
                        })
                    }
                    "ranged" => {
                        eb = eb.with(Ranged {
                            range: effect.1.parse::<i32>().unwrap(),
                        })
                    }
                    "damage" => {
                        eb = eb.with(InflictsDamage {
                            damage: effect.1.parse::<i32>().unwrap(),
                        })
                    }
                    "area_of_effect" => {
                        eb = eb.with(AreaOfEffect {
                            radius: effect.1.parse::<i32>().unwrap(),
                        })
                    }
                    "confusion" => {
                        eb = eb.with(Confusion {
                            turns: effect.1.parse::<i32>().unwrap(),
                        })
                    }
                    "magic_mapping" => eb = eb.with(MagicMapper {}),
                    "food" => eb = eb.with(ProvidesFood {}),
                    _ => console::log(format!(
                        "Warning: consumable effect {effect_name} not implemented"
                    )),
                }
            }
            return Some(eb.build());
        }

        if let Some(weapon) = &item_template.weapon {
            eb = eb.with(Equippable {
                slot: EquipmentSlot::Melee,
            });
            let roll: DiceRoll = weapon.base_damage.parse().unwrap();
            let mut wpn = MeleeWeapon {
                attribute: WeaponAttribute::Might,
                hit_bonus: weapon.hit_bonus,
                damage_n_dice: roll.n_dice,
                damage_die_type: roll.die_type,
                damage_bonus: roll.die_bonus,
            };
            match weapon.attribute.as_str() {
                "Quickness" | "quickness" => wpn.attribute = WeaponAttribute::Quickness,
                "Might" | "might" => wpn.attribute = WeaponAttribute::Might,
                "Fitness" | "fitness" => {}
                unknown => unreachable!("Unknown attribute: {unknown}"),
            }
            eb = eb.with(wpn);
        }

        if let Some(wearable) = &item_template.wearable {
            let slot = wearable.slot.parse().unwrap();
            eb = eb.with(Equippable { slot });
            eb = eb.with(Wearable {
                slot,
                armor_class: wearable.armor_class,
            });
        }

        return Some(eb.build());
    }

    None
}

pub fn spawn_named_mob(
    raws: &RawMaster,
    ecs: &mut World,
    key: &str,
    pos: SpawnType,
) -> Option<Entity> {
    if raws.mob_index.contains_key(key) {
        let mob_template = &raws.raws.mobs[raws.mob_index[key]];
        let mut eb = ecs.create_entity().marked::<SimpleMarker<SerializeMe>>();
        eb = spawn_position(pos, eb, key, raws);

        if let Some(renderable) = &mob_template.renderable {
            eb = eb.with(get_renderable_component(renderable));
        }

        if let Some(quips) = &mob_template.quips {
            eb = eb.with(Quips {
                available: quips.clone(),
            });
        }

        eb = eb.with(Name {
            name: mob_template.name.clone(),
        });

        if mob_template.blocks_tile {
            eb = eb.with(BlocksTile {});
        }

        eb = eb.with(Viewshed {
            visible_tiles: Vec::new(),
            range: mob_template.vision_range,
            dirty: true,
        });

        let mut mob_fitness = 11;
        let mut mob_int = 11;
        let mut attr = Attributes {
            might: Attribute {
                base: 11,
                modifiers: 0,
                bonus: attr_bonus(11),
            },
            fitness: Attribute {
                base: 11,
                modifiers: 0,
                bonus: attr_bonus(11),
            },
            quickness: Attribute {
                base: 11,
                modifiers: 0,
                bonus: attr_bonus(11),
            },
            intelligence: Attribute {
                base: 11,
                modifiers: 0,
                bonus: attr_bonus(11),
            },
        };
        if let Some(might) = mob_template.attributes.might {
            attr.might = Attribute {
                base: might,
                modifiers: 0,
                bonus: attr_bonus(might),
            };
        }
        if let Some(fitness) = mob_template.attributes.fitness {
            attr.fitness = Attribute {
                base: fitness,
                modifiers: 0,
                bonus: attr_bonus(fitness),
            };
            mob_fitness = fitness;
        }
        if let Some(quickness) = mob_template.attributes.quickness {
            attr.quickness = Attribute {
                base: quickness,
                modifiers: 0,
                bonus: attr_bonus(quickness),
            };
        }
        if let Some(intelligence) = mob_template.attributes.intelligence {
            attr.intelligence = Attribute {
                base: intelligence,
                modifiers: 0,
                bonus: attr_bonus(intelligence),
            };
            mob_int = intelligence;
        }
        eb = eb.with(attr);

        let mob_level = if mob_template.level.is_some() {
            mob_template.level.unwrap()
        } else {
            1
        };
        let mob_hp = npc_hp(mob_fitness, mob_level);
        let mob_mana = mana_at_level(mob_int, mob_level);

        let pools = Pools {
            level: mob_level,
            xp: 0,
            hit_points: Pool {
                current: mob_hp,
                max: mob_hp,
            },
            mana: Pool {
                current: mob_mana,
                max: mob_mana,
            },
            total_weight: 0.0,
            total_initiative_penalty: 0.0,
            gold: mob_template.gold.as_ref().map_or(0.0, |gold| {
                let mut rng = RandomNumberGenerator::new();
                let roll: DiceRoll = gold.parse().unwrap();
                (rng.roll_dice(roll.n_dice, roll.die_type) + roll.die_bonus) as f32
            }),
            god_mode: false,
        };
        eb = eb.with(pools);
        eb = eb.with(EquipmentChanged {});

        let mut skills = Skills {
            skills: HashMap::new(),
        };
        skills.skills.insert(Skill::Melee, 1);
        skills.skills.insert(Skill::Defense, 1);
        skills.skills.insert(Skill::Magic, 1);
        if let Some(mobskills) = &mob_template.skills {
            for sk in mobskills.iter() {
                match sk.0.as_str() {
                    "Melee" => {
                        skills.skills.insert(Skill::Melee, *sk.1);
                    }
                    "Defense" => {
                        skills.skills.insert(Skill::Defense, *sk.1);
                    }
                    "Magic" => {
                        skills.skills.insert(Skill::Magic, *sk.1);
                    }
                    _ => {
                        unreachable!("Unknown skill referenced: [{}]", sk.0);
                    }
                }
            }
        }
        eb = eb.with(skills);

        if let Some(vendor) = &mob_template.vendor {
            eb = eb.with(Vendor {
                categories: vendor.clone(),
            });
        }

        if let Some(na) = &mob_template.natural {
            let mut nature = NaturalAttackDefense {
                armor_class: na.armor_class,
                attacks: Vec::new(),
            };
            if let Some(attacks) = &na.attacks {
                for nattack in attacks.iter() {
                    let roll: DiceRoll = nattack.damage.parse().unwrap();
                    let attack = NaturalAttack {
                        name: nattack.name.clone(),
                        hit_bonus: nattack.hit_bonus,
                        damage_n_dice: roll.n_dice,
                        damage_die_type: roll.die_type,
                        damage_bonus: roll.die_bonus,
                    };
                    nature.attacks.push(attack);
                }
            }
            eb = eb.with(nature);
        }

        if let Some(loot) = &mob_template.loot_table {
            eb = eb.with(LootTable {
                table: loot.clone(),
            });
        }

        if let Some(light) = &mob_template.light {
            eb = eb.with(LightSource {
                range: light.range,
                color: RGB::from_hex(&light.color).expect("Bad color"),
            });
        }

        if let Some(faction) = &mob_template.faction {
            eb = eb.with(Faction {
                name: faction.clone(),
            });
        } else {
            eb = eb.with(Faction {
                name: "Mindless".to_string(),
            })
        }

        match mob_template.movement.as_ref() {
            "random" => {
                eb = eb.with(MoveMode {
                    mode: Movement::Random,
                })
            }
            "random_waypoint" => {
                eb = eb.with(MoveMode {
                    mode: Movement::RandomWaypoint { path: None },
                })
            }
            _ => {
                eb = eb.with(MoveMode {
                    mode: Movement::Static,
                })
            }
        }

        eb = eb.with(Initiative { current: 2 });

        let new_mob = eb.build();

        if let Some(wielding) = &mob_template.equipped {
            for tag in wielding.iter() {
                spawn_named_entity(raws, ecs, tag, SpawnType::Equipped { by: new_mob });
            }
        }

        return Some(new_mob);
    }

    None
}

pub fn spawn_named_entity(
    raws: &RawMaster,
    ecs: &mut World,
    key: &str,
    pos: SpawnType,
) -> Option<Entity> {
    if raws.item_index.contains_key(key) {
        return spawn_named_item(raws, ecs, key, pos);
    } else if raws.mob_index.contains_key(key) {
        return spawn_named_mob(raws, ecs, key, pos);
    } else if raws.prop_index.contains_key(key) {
        return spawn_named_prop(raws, ecs, key, pos);
    }

    None
}

pub fn spawn_named_prop(
    raws: &RawMaster,
    ecs: &mut World,
    key: &str,
    pos: SpawnType,
) -> Option<Entity> {
    if raws.prop_index.contains_key(key) {
        let prop_template = &raws.raws.props[raws.prop_index[key]];

        let mut eb = ecs.create_entity().marked::<SimpleMarker<SerializeMe>>();

        eb = spawn_position(pos, eb, key, raws);

        if let Some(renderable) = &prop_template.renderable {
            eb = eb.with(get_renderable_component(renderable));
        }

        eb = eb.with(Name {
            name: prop_template.name.clone(),
        });

        if let Some(hidden) = prop_template.hidden {
            if hidden {
                eb = eb.with(Hidden {})
            };
        }

        if let Some(blocks_tile) = prop_template.blocks_tile {
            if blocks_tile {
                eb = eb.with(BlocksTile {})
            };
        }
        if let Some(blocks_visibility) = prop_template.blocks_visibility {
            if blocks_visibility {
                eb = eb.with(BlocksVisibility {})
            };
        }
        if let Some(door_open) = prop_template.door_open {
            if door_open {
                eb = eb.with(Door { open: door_open })
            };
        }
        if let Some(entry_trigger) = &prop_template.entry_trigger {
            eb = eb.with(EntryTrigger {});
            for effect in entry_trigger.effects.iter() {
                match effect.0.as_str() {
                    "damage" => {
                        eb = eb.with(InflictsDamage {
                            damage: effect.1.parse::<i32>().unwrap(),
                        })
                    }
                    "single_activation" => eb = eb.with(SingleActivation {}),
                    _ => {}
                }
            }
        }

        if let Some(light) = &prop_template.light {
            eb = eb.with(LightSource {
                range: light.range,
                color: RGB::from_hex(&light.color).expect("Bad Color"),
            });
            eb = eb.with(Viewshed {
                range: light.range,
                dirty: true,
                visible_tiles: vec![],
            });
        }

        return Some(eb.build());
    }

    None
}

pub fn get_spawn_table_for_depth(raws: &RawMaster, depth: i32) -> RandomTable {
    let available_options: Vec<&SpawnTableEntry> = raws
        .raws
        .spawn_table
        .iter()
        .filter(|a| depth >= a.min_depth && depth <= a.max_depth)
        .collect();

    let mut rt = RandomTable::new();
    for e in available_options.iter() {
        let mut weight = e.weight;
        if e.add_map_depth_to_weight.is_some() {
            weight += depth;
        }
        rt = rt.add(e.name.clone(), weight);
    }

    rt
}

fn find_slot_for_equippable_item(tag: &str, raws: &RawMaster) -> EquipmentSlot {
    if !raws.item_index.contains_key(tag) {
        panic!("Trying to equip an unknown item: {tag}");
    }
    let item_index = raws.item_index[tag];
    let item = &raws.raws.items[item_index];
    if let Some(_wpn) = &item.weapon {
        return EquipmentSlot::Melee;
    } else if let Some(wearable) = &item.wearable {
        return wearable.slot.parse::<EquipmentSlot>().unwrap();
    }
    panic!("Trying to equip {tag}, but it has no slot tag.");
}

pub fn get_item_drop(
    raws: &RawMaster,
    rng: &mut RandomNumberGenerator,
    table: &str,
) -> Option<String> {
    if raws.loot_index.contains_key(table) {
        let mut rt = RandomTable::new();
        let available_options = &raws.raws.loot_tables[raws.loot_index[table]];
        for item in available_options.drops.iter() {
            rt = rt.add(item.name.clone(), item.weight);
        }
        return Some(rt.roll(rng));
    }

    None
}

pub fn faction_reaction(my_faction: &str, their_faction: &str, raws: &RawMaster) -> Reaction {
    if raws.faction_index.contains_key(my_faction) {
        let mf = &raws.faction_index[my_faction];
        return if mf.contains_key(their_faction) {
            mf[their_faction]
        } else if mf.contains_key("Default") {
            mf["Default"]
        } else {
            Reaction::Ignore
        };
    };

    Reaction::Ignore
}

pub fn get_vendor_items(categories: &[String], raws: &RawMaster) -> Vec<(String, f32)> {
    let mut result = vec![];

    for item in raws.raws.items.iter() {
        if let Some(cat) = &item.vendor_category {
            if categories.contains(cat) && item.base_value.is_some() {
                result.push((item.name.clone(), item.base_value.unwrap()));
            }
        }
    }

    result
}
