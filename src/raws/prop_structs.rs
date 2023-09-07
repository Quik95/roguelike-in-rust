use std::collections::HashMap;

use serde::Deserialize;

use crate::raws::item_structs::Renderable;
use crate::raws::mob_structs;

#[derive(Deserialize, Debug)]
pub struct Prop {
    pub name: String,
    pub renderable: Option<Renderable>,
    pub hidden: Option<bool>,
    pub blocks_tile: Option<bool>,
    pub blocks_visibility: Option<bool>,
    pub door_open: Option<bool>,
    pub entry_trigger: Option<EntryTrigger>,
    pub light: Option<mob_structs::MobLight>,
}

#[derive(Deserialize, Debug)]
pub struct EntryTrigger {
    pub effects: HashMap<String, String>,
}
