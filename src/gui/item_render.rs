use crate::components::{Consumable, CursedItem, MagicItem, MagicItemClass, Name, ObfuscatedName};
use crate::map::dungeon::MasterDungeonMap;
use rltk::{RED, RGB};
use specs::{Entity, World, WorldExt};

pub fn get_item_color(ecs: &World, item: Entity) -> RGB {
    let dm = ecs.fetch::<MasterDungeonMap>();
    if let Some(name) = ecs.read_storage::<Name>().get(item) {
        if ecs.read_storage::<CursedItem>().get(item).is_some()
            && dm.identified_items.contains(&name.name)
        {
            return RED.into();
        }
    }

    if let Some(magic) = ecs.read_storage::<MagicItem>().get(item) {
        return match magic.class {
            MagicItemClass::Common => RGB::from_f32(0.5, 1.0, 0.5),
            MagicItemClass::Rare => RGB::from_f32(0.0, 1.0, 1.0),
            MagicItemClass::Legendary => RGB::from_f32(0.71, 0.15, 0.93),
        };
    }
    RGB::from_f32(1.0, 1.0, 1.0)
}

pub fn get_item_display_name(ecs: &World, item: Entity) -> String {
    ecs.read_storage::<Name>()
        .get(item)
        .map_or("Nameless item (bug)".into(), |name| {
            if ecs.read_storage::<MagicItem>().get(item).is_some() {
                let dm = ecs.fetch::<MasterDungeonMap>();
                if dm.identified_items.contains(&name.name) {
                    if let Some(c) = ecs.read_storage::<Consumable>().get(item) {
                        if c.max_charges > 1 {
                            format!("{} ({})", name.name.clone(), c.charges)
                        } else {
                            name.name.clone()
                        }
                    } else {
                        name.name.clone()
                    }
                } else if let Some(obfuscated) = ecs.read_storage::<ObfuscatedName>().get(item) {
                    obfuscated.name.clone()
                } else {
                    "Unidentified magic item".to_string()
                }
            } else {
                name.name.clone()
            }
        })
}
