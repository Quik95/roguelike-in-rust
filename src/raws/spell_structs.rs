use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Spell {
    pub name: String,
    pub effects: HashMap<String, String>,
    pub mana_cost: i32,
}
