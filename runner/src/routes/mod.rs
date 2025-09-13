type AppState = axum::extract::State<std::sync::Arc<crate::AppState>>;

mod start;

pub use start::start;

mod stop;
pub use stop::stop;

mod running;
pub use running::running;

mod stats;
pub use stats::stats;

mod ip;
pub use ip::ip;

mod list;
pub use list::list;

mod exec;
pub use exec::exec;

mod ping;
pub use ping::ping;

mod console;
pub use console::console;
