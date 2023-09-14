use rltk::{to_cp437, RandomNumberGenerator, BLACK, BLUE, CYAN};
use specs::{Entities, Entity, Join, ReadStorage, System, WriteExpect, WriteStorage};

use crate::components::{
    Attributes, EquipmentSlot, Equipped, HungerClock, HungerState, Name, NaturalAttackDefense,
    Pools, Skill, Skills, WantsToMelee, Weapon, WeaponAttribute, Wearable,
};
use crate::effects::{add_effect, EffectType, Targets};
use crate::gamelog::GameLog;
use crate::gamesystem::skill_bonus;

pub struct MeleeCombatSystem {}

impl<'a> System<'a> for MeleeCombatSystem {
    type SystemData = (
        Entities<'a>,
        WriteExpect<'a, GameLog>,
        WriteStorage<'a, WantsToMelee>,
        ReadStorage<'a, Name>,
        ReadStorage<'a, Attributes>,
        ReadStorage<'a, Skills>,
        ReadStorage<'a, HungerClock>,
        ReadStorage<'a, Pools>,
        WriteExpect<'a, RandomNumberGenerator>,
        ReadStorage<'a, Equipped>,
        ReadStorage<'a, Weapon>,
        ReadStorage<'a, Wearable>,
        ReadStorage<'a, NaturalAttackDefense>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut log,
            mut wants_melee,
            names,
            attributes,
            skills,
            hunger_clock,
            pools,
            mut rng,
            equipped_items,
            meleeweapons,
            wearables,
            natural,
        ) = data;

        for (entity, wants_melee, name, attacker_attributes, attacker_skills, attacker_pools) in (
            &entities,
            &wants_melee,
            &names,
            &attributes,
            &skills,
            &pools,
        )
            .join()
        {
            let target_pools = pools.get(wants_melee.target).unwrap();
            let target_attributes = attributes.get(wants_melee.target).unwrap();
            let target_skills = skills.get(wants_melee.target).unwrap();
            if attacker_pools.hit_points.current > 0 && target_pools.hit_points.current > 0 {
                let target_name = names.get(wants_melee.target).unwrap();

                let mut weapon_info = Weapon {
                    attribute: WeaponAttribute::Might,
                    damage_n_dice: 1,
                    damage_die_type: 4,
                    ..Default::default()
                };

                if let Some(nat) = natural.get(entity) {
                    if !nat.attacks.is_empty() {
                        let attack_index = if nat.attacks.len() == 1 {
                            0
                        } else {
                            rng.roll_dice(1, nat.attacks.len() as i32) - 1
                        } as usize;
                        weapon_info.hit_bonus = nat.attacks[attack_index].hit_bonus;
                        weapon_info.damage_n_dice = nat.attacks[attack_index].damage_n_dice;
                        weapon_info.damage_die_type = nat.attacks[attack_index].damage_die_type;
                        weapon_info.damage_bonus = nat.attacks[attack_index].damage_bonus;
                    }
                }

                let mut weapon: Option<Entity> = None;
                for (weapon_entity, wielded, melee) in
                    (&entities, &equipped_items, &meleeweapons).join()
                {
                    if wielded.owner == entity && wielded.slot == EquipmentSlot::Melee {
                        weapon_info = melee.clone();
                        weapon = Some(weapon_entity);
                    }
                }

                let natural_roll = rng.roll_dice(1, 20);
                let attribute_hit_bonus = if weapon_info.attribute == WeaponAttribute::Might {
                    attacker_attributes.might.bonus
                } else {
                    attacker_attributes.quickness.bonus
                };
                let skill_hit_bonus = skill_bonus(Skill::Melee, attacker_skills);
                let weapon_hit_bonus = weapon_info.hit_bonus;
                let mut status_hit_bonus = 0;
                if let Some(hc) = hunger_clock.get(entity) {
                    if hc.state == HungerState::WellFed {
                        status_hit_bonus += 1;
                    }
                }
                let modified_hit_roll = natural_roll
                    + attribute_hit_bonus
                    + skill_hit_bonus
                    + weapon_hit_bonus
                    + status_hit_bonus;

                let armor_item_bonus_f: f32 = (&equipped_items, &wearables)
                    .join()
                    .filter_map(|(wielded, armor)| {
                        if wielded.owner == wants_melee.target {
                            Some(armor.armor_class)
                        } else {
                            None
                        }
                    })
                    .sum();
                let base_armor_class = natural
                    .get(wants_melee.target)
                    .map_or(10, |nat| nat.armor_class.unwrap_or(10));
                let armor_quickness_bonus = target_attributes.quickness.bonus;
                let armor_skill_bonus = skill_bonus(Skill::Defense, target_skills);
                let armor_item_bonus = armor_item_bonus_f as i32;
                let armor_class =
                    base_armor_class + armor_quickness_bonus + armor_skill_bonus + armor_item_bonus;

                if natural_roll != 1 && (natural_roll == 20 || modified_hit_roll > armor_class) {
                    let base_damage =
                        rng.roll_dice(weapon_info.damage_n_dice, weapon_info.damage_die_type);
                    let attr_damage_bonus = attacker_attributes.might.bonus;
                    let skill_damage_bonus = skill_bonus(Skill::Melee, attacker_skills);
                    let weapon_damage_bonus = weapon_info.damage_bonus;

                    let damage = i32::max(
                        0,
                        base_damage
                            + attr_damage_bonus
                            + skill_hit_bonus
                            + skill_damage_bonus
                            + weapon_damage_bonus,
                    );
                    add_effect(
                        Some(entity),
                        EffectType::Damage { amount: damage },
                        Targets::Single {
                            target: wants_melee.target,
                        },
                    );
                    log.entries.push(format!(
                        "{} hits {}, for {damage} hp.",
                        &name.name, &target_name.name
                    ));

                    if let Some(chance) = &weapon_info.proc_chance {
                        if rng.roll_dice(1, 100) <= (chance * 100.0) as i32 {
                            let effect_target = if weapon_info.proc_target.unwrap() == "Self" {
                                Targets::Single { target: entity }
                            } else {
                                Targets::Single {
                                    target: wants_melee.target,
                                }
                            };
                            add_effect(
                                Some(entity),
                                EffectType::ItemUse {
                                    item: weapon.unwrap(),
                                },
                                effect_target,
                            );
                        }
                    }
                } else if natural_roll == 1 {
                    log.entries.push(format!(
                        "{} considers attarcking {}, but misjudges the timing.",
                        name.name, target_name.name
                    ));
                    add_effect(
                        None,
                        EffectType::Particle {
                            glyph: to_cp437('‼'),
                            fg: BLUE.into(),
                            bg: BLACK.into(),
                            lifespan: 200.0,
                        },
                        Targets::Single {
                            target: wants_melee.target,
                        },
                    );
                } else {
                    log.entries.push(format!(
                        "{} attacks {}, but can't connect.",
                        name.name, target_name.name
                    ));
                    add_effect(
                        None,
                        EffectType::Particle {
                            glyph: to_cp437('‼'),
                            fg: CYAN.into(),
                            bg: BLACK.into(),
                            lifespan: 200.0,
                        },
                        Targets::Single {
                            target: wants_melee.target,
                        },
                    );
                }
            }
        }
        wants_melee.clear();
    }
}
