use specs::{Entity, ReadStorage};

pub use collection_system::ItemCollectionSystem;
pub use drop_system::ItemDropSystem;
pub use equip_system::ItemEquipOnUse;
pub use identification_system::ItemIdentificationSystem;
pub use item_use_system::ItemUseSystem;
pub use remove_system::ItemRemoveSystem;
pub use spell_system::SpellUseSystem;

use crate::components::{MagicItem, Name, ObfuscatedName};
use crate::map::dungeon::MasterDungeonMap;

mod collection_system;
mod drop_system;
mod equip_system;
mod identification_system;
mod item_use_system;
mod remove_system;
mod spell_system;

fn obfuscate_name(
    item: Entity,
    names: &ReadStorage<Name>,
    magic_items: &ReadStorage<MagicItem>,
    obfuscated_names: &ReadStorage<ObfuscatedName>,
    dm: &MasterDungeonMap,
) -> String {
    names
        .get(item)
        .map_or("Nameless item (bug)".into(), |name| {
            if magic_items.get(item).is_some() {
                if dm.identified_items.contains(&name.name) {
                    name.name.clone()
                } else if let Some(obfuscated) = obfuscated_names.get(item) {
                    obfuscated.name.clone()
                } else {
                    "Unidentified magic item".into()
                }
            } else {
                name.name.clone()
            }
        })
}
