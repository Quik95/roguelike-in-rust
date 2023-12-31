use std::str;

use serde::Deserialize;

pub use faction_structs::Reaction;

use crate::raws::faction_structs::FactionInfo;
use crate::raws::item_structs::Item;
use crate::raws::loot_structs::LootTable;
use crate::raws::mob_structs::Mob;
use crate::raws::prop_structs::Prop;
use crate::raws::rawmaster::RAWS;
use crate::raws::spawn_table_structs::SpawnTableEntry;
use crate::raws::spell_structs::Spell;

pub use weapon_traits::*;

mod faction_structs;
mod item_structs;
mod loot_structs;
mod mob_structs;
mod prop_structs;
pub mod rawmaster;
mod spawn_table_structs;
mod spell_structs;
mod weapon_traits;

rltk::embedded_resource!(RAW_FILE, "../../raws/spawns.json");

pub fn load_raws() {
    rltk::link_resource!(RAW_FILE, "../../raws/spawns.json");

    let raw_data = rltk::embedding::EMBED
        .lock()
        .get_resource("../../raws/spawns.json".to_string())
        .unwrap();
    let raw_string = str::from_utf8(raw_data).expect("Unable to convert to a valid UTF-8 string.");
    let decoder: Raws = serde_json::from_str(raw_string).expect("Unable to parse JSON");

    RAWS.lock().unwrap().load(decoder);
}

#[derive(Deserialize, Debug, Default)]
pub struct Raws {
    pub items: Vec<Item>,
    pub mobs: Vec<Mob>,
    pub props: Vec<Prop>,
    pub spawn_table: Vec<SpawnTableEntry>,
    pub loot_tables: Vec<LootTable>,
    pub faction_table: Vec<FactionInfo>,
    pub spells: Vec<Spell>,
    pub weapon_traits: Vec<WeaponTrait>,
}
