mod start;

pub use start::start;

mod stop;
pub use stop::stop;

mod stats;
pub use stats::stats;

mod ip;
pub use ip::ip;

use reqwest::StatusCode;
async fn exec_cmd(
    mut stdin: RwLockWriteGuard<'_, Option<ChildStdin>>,
    cmd: &str,
) -> (StatusCode, &'static str) {
    if let Some(stdin) = stdin.as_mut() {
        let write_1 = stdin.write_all(cmd.as_bytes()).await;
        let write_2 = stdin.write_u8(b'\n').await;

        if let Err(err) = write_1.or(write_2) {
            tracing::warn!("failed to write to server stdin: {err}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to exec cmd.")
        } else {
            (StatusCode::OK, "executed cmd!")
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "server not on!")
    }
}

mod list;
pub use list::list;

mod exec;
pub use exec::exec;

mod ping;
pub use ping::ping;

mod console;
pub use console::console;
use tokio::{io::AsyncWriteExt, process::ChildStdin, sync::RwLockWriteGuard};
