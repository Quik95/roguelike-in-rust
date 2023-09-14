use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponTrait {
    pub name: String,
    pub effects: HashMap<String, String>,
}
