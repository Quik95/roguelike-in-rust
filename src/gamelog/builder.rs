use crate::gamelog::logstore::append_entry;
use crate::gamelog::LogFragment;
use rltk::{CYAN, RED, RGB, WHITE, YELLOW};

pub struct Logger {
    current_color: RGB,
    fragments: Vec<LogFragment>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            current_color: WHITE.into(),
            fragments: vec![],
        }
    }

    pub fn color(mut self, color: (u8, u8, u8)) -> Self {
        self.current_color = RGB::named(color);
        self
    }

    pub fn append(mut self, text: impl ToString) -> Self {
        self.fragments.push(LogFragment {
            color: self.current_color,
            text: text.to_string(),
        });
        self
    }

    pub fn log(self) {
        append_entry(self.fragments);
    }

    pub fn npc_name(mut self, text: impl ToString) -> Self {
        self.fragments.push(LogFragment {
            color: RGB::named(YELLOW),
            text: text.to_string(),
        });
        self
    }

    pub fn item_name(mut self, text: impl ToString) -> Self {
        self.fragments.push(LogFragment {
            color: RGB::named(CYAN),
            text: text.to_string(),
        });
        self
    }

    pub fn damage(mut self, damage: i32) -> Self {
        self.fragments.push(LogFragment {
            color: RGB::named(RED),
            text: format!("{damage}"),
        });
        self
    }
}
