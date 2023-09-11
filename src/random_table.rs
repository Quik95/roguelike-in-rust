use rltk::RandomNumberGenerator;

use crate::raws::rawmaster::{spawn_type_by_name, RawMaster, SpawnTableType};

pub struct RandomEntry {
    name: String,
    weight: i32,
}

impl RandomEntry {
    pub fn new<S: ToString>(name: S, weight: i32) -> Self {
        Self {
            name: name.to_string(),
            weight,
        }
    }
}

#[derive(Default)]
pub struct MasterTable {
    items: RandomTable,
    mobs: RandomTable,
    props: RandomTable,
}

impl MasterTable {
    pub fn add(&mut self, name: impl ToString, weight: i32, raws: &RawMaster) {
        match spawn_type_by_name(raws, &name.to_string()) {
            SpawnTableType::Item => self.items.add(name, weight),
            SpawnTableType::Mob => self.mobs.add(name, weight),
            SpawnTableType::Prop => self.props.add(name, weight),
        };
    }

    pub fn roll(&self, rng: &mut RandomNumberGenerator) -> String {
        let roll = rng.roll_dice(1, 4);
        match roll {
            1 => self.items.roll(rng),
            2 => self.props.roll(rng),
            3 => self.mobs.roll(rng),
            _ => "None".into(),
        }
    }
}

#[derive(Default)]
pub struct RandomTable {
    entries: Vec<RandomEntry>,
    total_weight: i32,
}

impl RandomTable {
    pub fn add<S: ToString>(&mut self, name: S, weight: i32) {
        if weight > 0 {
            self.total_weight += weight;
            self.entries.push(RandomEntry::new(name, weight));
        }
    }

    pub fn roll(&self, rng: &mut RandomNumberGenerator) -> String {
        if self.total_weight == 0 {
            return "None".to_string();
        }
        let mut roll = rng.roll_dice(1, self.total_weight) - 1;
        let mut index: usize = 0;

        while roll >= 0 {
            if roll < self.entries[index].weight {
                return self.entries[index].name.clone();
            }

            roll -= self.entries[index].weight;
            index += 1;
        }

        "None".to_string()
    }
}
