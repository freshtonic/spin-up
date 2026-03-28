const SPIN_CORE_NET: &str = include_str!("../spin-core-modules/spin-core-net.spin");

pub fn get_module_source(name: &str) -> Option<&'static str> {
    match name {
        "spin-core-net" => Some(SPIN_CORE_NET),
        _ => None,
    }
}

pub fn builtin_module_names() -> &'static [&'static str] {
    &["spin-core-net"]
}
