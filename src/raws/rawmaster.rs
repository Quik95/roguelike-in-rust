use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use lazy_static::lazy_static;
use rltk::{console, to_cp437, RandomNumberGenerator, RGB};
use specs::saveload::{MarkedBuilder, SimpleMarker};
use specs::{Builder, Entities, Entity, EntityBuilder, Join, ReadStorage, World, WorldExt};

use crate::components::{
    AreaOfEffect, Attribute, AttributeBonus, Attributes, BlocksTile, BlocksVisibility, Confusion,
    Consumable, CursedItem, DamageOverTime, Door, Duration, EntryTrigger, EquipmentChanged,
    EquipmentSlot, Equippable, Faction, Hidden, InBackpack, InflictsDamage, Initiative,
    LightSource, MagicItem, MagicItemClass, MagicMapper, MeleeWeapon, MoveMode, Movement, Name,
    NaturalAttack, NaturalAttackDefense, ObfuscatedName, Pool, Pools, Position, ProvidesFood,
    ProvidesHealing, ProvidesIdentification, ProvidesMana, ProvidesRemoveCurse, Ranged,
    SerializeMe, SingleActivation, Skill, Skills, Slow, SpawnParticleBurst, SpawnParticleLine,
    SpecialAbilities, SpecialAbility, SpellTemplate, TeachesSpell, TownPortal, Vendor, Viewshed,
    WeaponAttribute, Wearable,
};
use crate::components::{Equipped, LootTable};
use crate::components::{Quips, Renderable};
use crate::gamesystem::{attr_bonus, mana_at_level, npc_hp, DiceRoll};
use crate::map::dungeon::MasterDungeonMap;
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
    spell_index: HashMap<String, usize>,
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

        for spawn in &self.raws.spawn_table {
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

        for faction in &self.raws.faction_table {
            let mut reactions = HashMap::new();
            for other in &faction.responses {
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

        for (i, spell) in self.raws.spells.iter().enumerate() {
            self.spell_index.insert(spell.name.clone(), i);
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

macro_rules! apply_effects {
    ( $effects:expr, $eb:expr ) => {
        for effect in $effects.iter() {
            let effect_name = effect.0.as_str();
            match effect_name {
                "provides_healing" => {
                    $eb = $eb.with(ProvidesHealing {
                        heal_amount: effect.1.parse::<i32>().unwrap(),
                    })
                }
                "provides_mana" => {
                    $eb = $eb.with(ProvidesMana {
                        mana_amount: effect.1.parse::<i32>().unwrap(),
                    })
                }
                "ranged" => {
                    $eb = $eb.with(Ranged {
                        range: effect.1.parse::<i32>().unwrap(),
                    })
                }
                "damage" => {
                    $eb = $eb.with(InflictsDamage {
                        damage: effect.1.parse::<i32>().unwrap(),
                    })
                }
                "area_of_effect" => {
                    $eb = $eb.with(AreaOfEffect {
                        radius: effect.1.parse::<i32>().unwrap(),
                    })
                }
                "confusion" => {
                    $eb = $eb.with(Confusion {});
                    $eb = $eb.with(Duration {
                        turns: effect.1.parse::<i32>().unwrap(),
                    });
                }
                "magic_mapping" => $eb = $eb.with(MagicMapper {}),
                "town_portal" => $eb = $eb.with(TownPortal {}),
                "food" => $eb = $eb.with(ProvidesFood {}),
                "single_activation" => $eb = $eb.with(SingleActivation {}),
                "particle_line" => {
                    $eb = $eb.with(
                        effect
                            .1
                            .parse::<SpawnParticleLine>()
                            .expect("Failed to parse SpawnParticleLine"),
                    )
                }
                "particle" => {
                    $eb = $eb.with(
                        effect
                            .1
                            .parse::<SpawnParticleBurst>()
                            .expect("Failed to parse SpawnParticleBurst"),
                    )
                }
                "remove_curse" => $eb = $eb.with(ProvidesRemoveCurse {}),
                "identify" => $eb = $eb.with(ProvidesIdentification {}),
                "teach_spell" => {
                    $eb = $eb.with(TeachesSpell {
                        spell: effect.1.into(),
                    })
                }
                "slow" => {
                    $eb = $eb.with(Slow {
                        initiative_penalty: effect.1.parse::<f32>().unwrap(),
                    })
                }
                "damage_over_time" => {
                    $eb = $eb.with(DamageOverTime {
                        damage: effect.1.parse::<i32>().unwrap(),
                    })
                }
                _ => console::log(format!(
                    "Warning: consumable effect {} not implemented.",
                    effect_name
                )),
            }
        }
    };
}

pub fn spawn_named_item(
    raws: &RawMaster,
    ecs: &mut World,
    key: &str,
    pos: SpawnType,
) -> Option<Entity> {
    if raws.item_index.contains_key(key) {
        let item_template = &raws.raws.items[raws.item_index[key]];

        let dm = ecs.fetch::<MasterDungeonMap>();
        let scroll_names = dm.scroll_mappings.clone();
        let potion_names = dm.potion_mappings.clone();
        let identified = dm.identified_items.clone();
        std::mem::drop(dm);

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
            let max_charges = consumable.charges.unwrap_or(1);
            eb = eb.with(Consumable {
                max_charges,
                charges: max_charges,
            });
            apply_effects!(consumable.effects, eb);
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
                proc_chance: weapon.proc_chance,
                proc_target: weapon.proc_target.clone(),
            };
            match weapon.attribute.as_str() {
                "Quickness" | "quickness" => wpn.attribute = WeaponAttribute::Quickness,
                "Might" | "might" => wpn.attribute = WeaponAttribute::Might,
                "Fitness" | "fitness" => {}
                unknown => unreachable!("Unknown attribute: {unknown}"),
            }
            eb = eb.with(wpn);
            if let Some(proc_effects) = &weapon.proc_effects {
                apply_effects!(proc_effects, eb);
            }
        }

        if let Some(wearable) = &item_template.wearable {
            let slot = wearable.slot.parse().unwrap();
            eb = eb.with(Equippable { slot });
            eb = eb.with(Wearable {
                slot,
                armor_class: wearable.armor_class,
            });
        }

        if let Some(magic) = &item_template.magic {
            let class = match magic.class.as_str() {
                "rare" => MagicItemClass::Rare,
                "legendary" => MagicItemClass::Legendary,
                "common" => MagicItemClass::Common,
                unknown => unreachable!("Unknown magic level: {unknown}"),
            };
            eb = eb.with(MagicItem { class });

            if !identified.contains(&item_template.name) {
                match magic.naming.as_str() {
                    "scroll" => {
                        eb = eb.with(ObfuscatedName {
                            name: scroll_names[&item_template.name].clone(),
                        });
                    }
                    "potion" => {
                        eb = eb.with(ObfuscatedName {
                            name: potion_names[&item_template.name].clone(),
                        });
                    }
                    _ => {
                        eb = eb.with(ObfuscatedName {
                            name: magic.naming.clone(),
                        });
                    }
                }
            }

            if let Some(cursed) = magic.cursed {
                if cursed {
                    eb = eb.with(CursedItem {});
                }
            }
        }

        if let Some(ab) = &item_template.attributes {
            eb = eb.with(AttributeBonus {
                might: ab.might,
                fitness: ab.fitness,
                quickness: ab.quickness,
                intelligence: ab.intelligence,
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
            for sk in mobskills {
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
                for nattack in attacks {
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
            });
        }

        match mob_template.movement.as_ref() {
            "random" => {
                eb = eb.with(MoveMode {
                    mode: Movement::Random,
                });
            }
            "random_waypoint" => {
                eb = eb.with(MoveMode {
                    mode: Movement::RandomWaypoint { path: None },
                });
            }
            _ => {
                eb = eb.with(MoveMode {
                    mode: Movement::Static,
                });
            }
        }

        eb = eb.with(Initiative { current: 2 });

        if let Some(ability_list) = &mob_template.abilities {
            let mut a = SpecialAbilities { abilities: vec![] };
            for ability in ability_list.iter() {
                a.abilities.push(SpecialAbility {
                    spell: ability.spell.clone(),
                    chance: ability.chance,
                    range: ability.range,
                    min_range: ability.min_range,
                });
            }
        }

        let new_mob = eb.build();

        if let Some(wielding) = &mob_template.equipped {
            for tag in wielding {
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
                eb = eb.with(Hidden {});
            };
        }

        if let Some(blocks_tile) = prop_template.blocks_tile {
            if blocks_tile {
                eb = eb.with(BlocksTile {});
            };
        }
        if let Some(blocks_visibility) = prop_template.blocks_visibility {
            if blocks_visibility {
                eb = eb.with(BlocksVisibility {});
            };
        }
        if let Some(door_open) = prop_template.door_open {
            if door_open {
                eb = eb.with(Door { open: door_open });
            };
        }
        if let Some(entry_trigger) = &prop_template.entry_trigger {
            eb = eb.with(EntryTrigger {});
            apply_effects!(entry_trigger.effects, eb);
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
    for e in &available_options {
        let mut weight = e.weight;
        if e.add_map_depth_to_weight.is_some() {
            weight += depth;
        }
        rt = rt.add(e.name.clone(), weight);
    }

    rt
}

fn find_slot_for_equippable_item(tag: &str, raws: &RawMaster) -> EquipmentSlot {
    assert!(
        raws.item_index.contains_key(tag),
        "Trying to equip an unknown item: {tag}"
    );
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
        for item in &available_options.drops {
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

    for item in &raws.raws.items {
        if let Some(cat) = &item.vendor_category {
            if categories.contains(cat) && item.base_value.is_some() {
                result.push((item.name.clone(), item.base_value.unwrap()));
            }
        }
    }

    result
}

pub fn get_scroll_tags() -> Vec<String> {
    let raws = &RAWS.lock().unwrap();
    let mut result = vec![];

    for item in &raws.raws.items {
        if let Some(magic) = &item.magic {
            if &magic.naming == "scroll" {
                result.push(item.name.clone());
            }
        }
    }

    result
}

pub fn is_tag_magic(tag: &str) -> bool {
    let raws = &RAWS.lock().unwrap();
    if raws.item_index.contains_key(tag) {
        let item_template = &raws.raws.items[raws.item_index[tag]];
        item_template.magic.is_some()
    } else {
        false
    }
}

pub fn get_potion_tag() -> Vec<String> {
    let raws = &RAWS.lock().unwrap();
    let mut result = Vec::new();

    for item in &raws.raws.items {
        if let Some(magic) = &item.magic {
            if &magic.naming == "potion" {
                result.push(item.name.clone());
            }
        }
    }

    result
}

pub fn spawn_named_spell(raws: &RawMaster, ecs: &mut World, key: &str) -> Option<Entity> {
    if !raws.spell_index.contains_key(key) {
        return None;
    }
    let spell_template = &raws.raws.spells[raws.spell_index[key]];

    let mut eb = ecs.create_entity().marked::<SimpleMarker<SerializeMe>>();
    eb = eb.with(SpellTemplate {
        mana_cost: spell_template.mana_cost,
    });
    eb = eb.with(Name {
        name: spell_template.name.clone(),
    });
    apply_effects!(spell_template.effects, eb);

    Some(eb.build())
}

pub fn spawn_all_spells(ecs: &mut World) {
    let raws = &super::RAWS.lock().unwrap();
    for spell in raws.raws.spells.iter() {
        spawn_named_spell(raws, ecs, &spell.name);
    }
}

pub fn find_spell_entity(ecs: &World, name: &str) -> Option<Entity> {
    let names = ecs.read_storage::<Name>();
    let spell_templates = ecs.read_storage::<SpellTemplate>();
    let entities = ecs.entities();

    for (entity, spell_name, _template) in (&entities, &names, &spell_templates).join() {
        if name == spell_name.name {
            return Some(entity);
        }
    }

    None
}

pub fn find_spell_entity_by_name(
    name: &str,
    names: &ReadStorage<Name>,
    spell_templates: &ReadStorage<SpellTemplate>,
    entities: &Entities,
) -> Option<Entity> {
    for (entity, sname, _template) in (entities, names, spell_templates).join() {
        if name == sname.name {
            return Some(entity);
        }
    }

    None
}
