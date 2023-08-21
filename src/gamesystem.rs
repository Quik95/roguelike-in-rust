use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;

use crate::components::{Skill, Skills};

pub const fn attr_bonus(value: i32) -> i32 {
    (value - 10) / 2
}

pub const fn player_hp_per_level(fitness: i32) -> i32 {
    10 + attr_bonus(fitness)
}

pub const fn player_hp_at_level(fitness: i32, level: i32) -> i32 {
    player_hp_per_level(fitness) * level
}

pub fn npc_hp(fitness: i32, level: i32) -> i32 {
    1 + (0..level)
        .into_iter()
        .map(|_| i32::max(1, 8 + attr_bonus(fitness)))
        .sum::<i32>()
}

pub fn mana_per_level(intelligence: i32) -> i32 {
    i32::max(1, 4 + attr_bonus(intelligence))
}

pub fn mana_at_level(intelligence: i32, level: i32) -> i32 {
    mana_per_level(intelligence) * level
}

pub fn skill_bonus(skill: Skill, skills: &Skills) -> i32 {
    if skills.skills.contains_key(&skill) {
        skills.skills[&skill]
    } else {
        -4
    }
}

pub struct DiceRoll {
    pub n_dice: i32,
    pub die_type: i32,
    pub die_bonus: i32,
}

impl FromStr for DiceRoll {
    type Err = !;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref DICE_REGEX: Regex = Regex::new(r"(\d+)d(\d+)([\+\-]\d+)?").unwrap();
        }
        let mut n_dice = 1;
        let mut die_type = 4;
        let mut die_bonus = 0;
        for cap in DICE_REGEX.captures_iter(s) {
            if let Some(group) = cap.get(1) {
                n_dice = group.as_str().parse::<i32>().expect("Not a digit");
            }
            if let Some(group) = cap.get(2) {
                die_type = group.as_str().parse::<i32>().expect("Not a digit");
            }
            if let Some(group) = cap.get(3) {
                die_bonus = group.as_str().parse::<i32>().expect("Not a digit");
            }
        }

        Ok(Self {
            n_dice,
            die_type,
            die_bonus,
        })
    }
}
