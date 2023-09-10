pub struct PrefabRoom {
    pub template: &'static str,
    pub width: usize,
    pub height: usize,
    pub first_depth: i32,
    pub last_depth: i32,
}

pub const TOTALLY_NOT_A_TRAP: PrefabRoom = PrefabRoom {
    template: TOTALLY_NOT_A_TRAP_MAP,
    width: 5,
    height: 5,
    first_depth: 0,
    last_depth: 100,
};

const TOTALLY_NOT_A_TRAP_MAP: &str = r#"      ^^^  ^!^  ^^^      "#;

pub const SILLY_SIMPLE: PrefabRoom = PrefabRoom {
    template: SILLY_SIMPLE_MAP,
    width: 6,
    height: 6,
    first_depth: 0,
    last_depth: 100,
};
pub const SILLY_SIMPLE_MAP: &str = r#"       ^  ^    #          ###       "#;

pub const CHECKERBOARD: PrefabRoom = PrefabRoom {
    template: CHECKERBOARD_MAP,
    width: 6,
    height: 6,
    first_depth: 0,
    last_depth: 100,
};
pub const CHECKERBOARD_MAP: &str = r#"       g#%#  #!#   ^# #             "#;
