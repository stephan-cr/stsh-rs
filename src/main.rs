pub mod execution;
pub mod parser;
pub mod sig;

use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::error::Error;

use crate::sig::{handler, install_sighandler, mask_sigchld, unmask_sigchld};

fn main() -> Result<(), Box<dyn Error>> {
    install_sighandler(libc::SIGCHLD, handler)?;

    if let Ok(chld_set) = mask_sigchld() {
        unmask_sigchld(chld_set)?;
    }

    let mut rl = Editor::<()>::new();

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                parser::parse(&line);
            },
            Err(ReadlineError::Eof) => {
                break;
            }
            _ => (),
        }
    }

    Ok(())
}
