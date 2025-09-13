#[macro_use]
mod macros;

make_forward!(start, "/start", crate::authorized::BasicAuth);

make_forward!(ip, "/ip", crate::authorized::BasicAuth);

make_forward!(stop, "/stop", crate::authorized::StopAuth);

make_forward!(running, "/running");

make_forward!(ping, "/ping");

make_forward!(list, "/list");

pub mod stats;

pub mod console;

pub mod wake;
