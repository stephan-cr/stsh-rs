pub mod execution;
pub mod sig;

use std::error::Error;

use crate::sig::{handler, install_sighandler, mask_sigchld, unmask_sigchld};

fn main() -> Result<(), Box<dyn Error>> {
    install_sighandler(libc::SIGCHLD, handler)?;

    if let Ok(chld_set) = mask_sigchld() {
        unmask_sigchld(chld_set)?;
    }

    Ok(())
}
