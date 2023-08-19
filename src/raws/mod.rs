use std::str;

use serde::Deserialize;

use crate::raws::item_structs::Item;
use crate::raws::mob_structs::Mob;
use crate::raws::prop_structs::Prop;
use crate::raws::rawmaster::RAWS;

mod item_structs;
pub mod rawmaster;
mod mob_structs;
mod prop_structs;

rltk::embedded_resource!(RAW_FILE, "../../raws/spawns.json");

pub fn load_raws() {
    rltk::link_resource!(RAW_FILE, "../../raws/spawns.json");

    let raw_data = rltk::embedding::EMBED
        .lock()
        .get_resource("../../raws/spawns.json".to_string())
        .unwrap();
    let raw_string = str::from_utf8(&raw_data).expect("Unable to convert to a valid UTF-8 string.");
    let decoder: Raws = serde_json::from_str(&raw_string).expect("Unable to parse JSON");

    RAWS.lock().unwrap().load(decoder);
}

#[derive(Deserialize, Debug)]
pub struct Raws {
    pub items: Vec<Item>,
    pub mobs: Vec<Mob>,
    pub props: Vec<Prop>,
}
