use std::fs;
use std::fs::File;
use std::path::Path;

#[allow(deprecated)]
use specs::error::NoError;
use specs::saveload::{
    DeserializeComponents, MarkedBuilder, SerializeComponents, SimpleMarker, SimpleMarkerAllocator,
};
use specs::{Builder, Entity, Join, World, WorldExt};

use crate::components::{
    AlwaysTargetsSelf, ApplyMove, ApplyTeleport, AreaOfEffect, AttributeBonus, Attributes,
    BlocksTile, BlocksVisibility, Chasing, Confusion, Consumable, CursedItem,
    DMSerializationHelper, DamageOverTime, DefenseBonus, Door, Duration, EntityMoved, EntryTrigger,
    EquipmentChanged, Equippable, Faction, Hidden, HungerClock, IdentifiedItem, InBackpack,
    InflictsDamage, Initiative, Item, KnownSpells, LightSource, LootTable, MagicItem, MagicMapper,
    MeleePowerBonus, MeleeWeapon, MoveMode, MyTurn, Name, NaturalAttackDefense, ObfuscatedName,
    OnDeath, OtherLevelPosition, ParticleLifetime, Player, Pools, Position, ProvidesFood,
    ProvidesHealing, ProvidesIdentification, ProvidesMana, ProvidesRemoveCurse, Quips, Ranged,
    Renderable, SingleActivation, Skills, Slow, SpawnParticleBurst, SpawnParticleLine,
    SpecialAbilities, SpellTemplate, StatusEffect, TeachesSpell, TeleportTo, TileSize, TownPortal,
    Vendor, Viewshed, WantsToApproach, WantsToCastSpell, WantsToDropItem, WantsToFlee,
    WantsToMelee, WantsToPickupItem, WantsToRemoveItem, WantsToUseItem, Wearable,
};
use crate::components::{SerializationHelper, SerializeMe};
use crate::map::dungeon::MasterDungeonMap;
use crate::spatial;

macro_rules! serialize_individually {
    ($ecs:expr, $ser:expr, $data:expr, $( $type:ty),*) => {
        $(
        SerializeComponents::<NoError, SimpleMarker<SerializeMe>>::serialize(
            &( $ecs.read_storage::<$type>(), ),
            &$data.0,
            &$data.1,
            &mut $ser,
        )
        .unwrap();
        )*
    };
}

#[allow(deprecated)]
pub fn save_game(ecs: &mut World) {
    let mapcopy = ecs.get_mut::<super::map::Map>().unwrap().clone();
    let dungeon_master = ecs.get_mut::<MasterDungeonMap>().unwrap().clone();
    let savehelper = ecs
        .create_entity()
        .with(SerializationHelper { map: mapcopy })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
    let savehelper2 = ecs
        .create_entity()
        .with(DMSerializationHelper {
            map: dungeon_master,
        })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
    {
        let data = (
            ecs.entities(),
            ecs.read_storage::<SimpleMarker<SerializeMe>>(),
        );
        let writer = File::create("/tmp/savegame.json").unwrap();
        let mut serializer = serde_json::Serializer::new(writer);
        serialize_individually!(
            ecs,
            serializer,
            data,
            Position,
            Renderable,
            Player,
            Viewshed,
            Name,
            BlocksTile,
            WantsToMelee,
            Item,
            Consumable,
            Ranged,
            InflictsDamage,
            AreaOfEffect,
            Confusion,
            ProvidesHealing,
            InBackpack,
            WantsToPickupItem,
            WantsToUseItem,
            WantsToDropItem,
            SerializationHelper,
            Equippable,
            MeleePowerBonus,
            DefenseBonus,
            WantsToRemoveItem,
            ParticleLifetime,
            HungerClock,
            ProvidesFood,
            MagicMapper,
            Hidden,
            EntryTrigger,
            EntityMoved,
            SingleActivation,
            Door,
            BlocksVisibility,
            Quips,
            Attributes,
            Skills,
            Pools,
            MeleeWeapon,
            Wearable,
            NaturalAttackDefense,
            LootTable,
            OtherLevelPosition,
            DMSerializationHelper,
            LightSource,
            Initiative,
            MyTurn,
            Faction,
            MoveMode,
            WantsToApproach,
            WantsToFlee,
            Chasing,
            EquipmentChanged,
            Vendor,
            TownPortal,
            TeleportTo,
            ApplyMove,
            ApplyTeleport,
            MagicItem,
            ObfuscatedName,
            IdentifiedItem,
            SpawnParticleLine,
            SpawnParticleBurst,
            CursedItem,
            ProvidesRemoveCurse,
            ProvidesIdentification,
            AttributeBonus,
            Duration,
            StatusEffect,
            KnownSpells,
            SpellTemplate,
            ProvidesMana,
            WantsToCastSpell,
            TeachesSpell,
            Slow,
            DamageOverTime,
            SpecialAbilities,
            TileSize,
            OnDeath,
            AlwaysTargetsSelf
        );
    }

    ecs.delete_entity(savehelper).expect("Crash on cleanup");
    ecs.delete_entity(savehelper2).expect("Crash on cleanup");
}

pub fn does_save_exist() -> bool {
    Path::new("/tmp/savegame.json").exists()
}

macro_rules! deserialize_individually {
    ($ecs:expr, $de:expr, $data:expr, $( $type:ty),*) => {
        $(
        DeserializeComponents::<NoError, _>::deserialize(
            &mut ( &mut $ecs.write_storage::<$type>(), ),
            // let's hope nothing breaks in the future
            $data.0, // entities
            &mut $data.1, // marker
            &mut $data.2, // allocater
            &mut $de,
        )
        .unwrap();
        )*
    };
}

#[allow(deprecated)]
pub fn load_game(ecs: &mut World) {
    {
        let mut to_delete = Vec::new();
        for e in ecs.entities().join() {
            to_delete.push(e);
        }
        for del in &to_delete {
            ecs.delete_entity(*del).expect("Deletion failed");
        }
    }

    let data = fs::read_to_string("/tmp/savegame.json").unwrap();
    let mut de = serde_json::Deserializer::from_str(&data);
    {
        let mut d = (
            &mut ecs.entities(),
            &mut ecs.write_storage::<SimpleMarker<SerializeMe>>(),
            &mut ecs.write_resource::<SimpleMarkerAllocator<SerializeMe>>(),
        );

        deserialize_individually!(
            ecs,
            de,
            d,
            Position,
            Renderable,
            Player,
            Viewshed,
            Name,
            BlocksTile,
            WantsToMelee,
            Item,
            Consumable,
            Ranged,
            InflictsDamage,
            AreaOfEffect,
            Confusion,
            ProvidesHealing,
            InBackpack,
            WantsToPickupItem,
            WantsToUseItem,
            WantsToDropItem,
            SerializationHelper,
            Equippable,
            MeleePowerBonus,
            DefenseBonus,
            WantsToRemoveItem,
            ParticleLifetime,
            HungerClock,
            ProvidesFood,
            MagicMapper,
            Hidden,
            EntryTrigger,
            EntityMoved,
            SingleActivation,
            Door,
            BlocksVisibility,
            Quips,
            Attributes,
            Skills,
            Pools,
            MeleeWeapon,
            Wearable,
            NaturalAttackDefense,
            LootTable,
            OtherLevelPosition,
            DMSerializationHelper,
            LightSource,
            Initiative,
            MyTurn,
            Faction,
            MoveMode,
            WantsToApproach,
            WantsToFlee,
            Chasing,
            EquipmentChanged,
            Vendor,
            TownPortal,
            TeleportTo,
            ApplyMove,
            ApplyTeleport,
            MagicItem,
            ObfuscatedName,
            IdentifiedItem,
            SpawnParticleLine,
            SpawnParticleBurst,
            CursedItem,
            ProvidesRemoveCurse,
            ProvidesIdentification,
            AttributeBonus,
            Duration,
            StatusEffect,
            KnownSpells,
            SpellTemplate,
            ProvidesMana,
            WantsToCastSpell,
            TeachesSpell,
            Slow,
            DamageOverTime,
            SpecialAbilities,
            TileSize,
            OnDeath,
            AlwaysTargetsSelf
        );
    }

    let mut deleteme: Option<Entity> = None;
    let mut deleteme2: Option<Entity> = None;
    {
        let entities = ecs.entities();
        let helper = ecs.read_storage::<SerializationHelper>();
        let helper2 = ecs.read_storage::<DMSerializationHelper>();
        let player = ecs.read_storage::<Player>();
        let position = ecs.read_storage::<Position>();
        for (e, h) in (&entities, &helper).join() {
            let mut worldmap = ecs.write_resource::<super::map::Map>();
            *worldmap = h.map.clone();
            spatial::set_size((worldmap.width * worldmap.height) as usize);
            deleteme = Some(e);
        }

        for (e, h) in (&entities, &helper2).join() {
            let mut dungeonmaster = ecs.write_resource::<MasterDungeonMap>();
            *dungeonmaster = h.map.clone();
            deleteme2 = Some(e);
        }

        for (e, _p, pos) in (&entities, &player, &position).join() {
            let mut ppos = ecs.write_resource::<rltk::Point>();
            *ppos = rltk::Point::new(pos.x, pos.y);
            let mut player_resource = ecs.write_resource::<Entity>();
            *player_resource = e;
        }
    }
    ecs.delete_entity(deleteme.unwrap())
        .expect("Unable to delete helper");
    ecs.delete_entity(deleteme2.unwrap())
        .expect("Unable to delete helper");
}

pub fn delete_save() {
    if Path::new("/tmp/savegame.json").exists() {
        fs::remove_file("/tmp/savegame.json").expect("Unable to delete file");
    }
}
