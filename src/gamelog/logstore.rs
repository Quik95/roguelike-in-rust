use crate::gamelog::LogFragment;
use lazy_static::lazy_static;
use rltk::{Console, Point, BLACK};
use std::sync::Mutex;
lazy_static! {
    static ref LOG: Mutex<Vec<Vec<LogFragment>>> = Mutex::new(Vec::new());
}

#[allow(dead_code)]
pub fn append_fragment(fragment: LogFragment) {
    LOG.lock().unwrap().push(vec![fragment]);
}

pub fn append_entry(fragments: Vec<LogFragment>) {
    LOG.lock().unwrap().push(fragments);
}

pub fn clear_log() {
    LOG.lock().unwrap().clear();
}

pub fn print_log(console: &mut Box<dyn Console>, pos: Point) {
    let mut x = pos.x;
    let mut y = pos.y;
    LOG.lock().unwrap().iter().rev().take(6).for_each(|log| {
        for frag in log {
            console.print_color(x, y, frag.color.into(), BLACK.into(), &frag.text);
            x += frag.text.len() as i32;
            x += 1;
        }
        y += 1;
        x = pos.x;
    });
}

pub fn clone_log() -> Vec<Vec<LogFragment>> {
    LOG.lock().unwrap().clone()
}

pub fn restore_log(log: &mut Vec<Vec<LogFragment>>) {
    LOG.lock().unwrap().clear();
    LOG.lock().unwrap().append(log);
}
