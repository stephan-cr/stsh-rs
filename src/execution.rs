use libc::{execvp, strerror};
use std::error::Error;
use std::ffi::{CStr, CString};
use std::fmt::{self, Display, Formatter};
use std::ptr::{null, null_mut};

use crate::parser::Command;

use libc::{
    __errno_location, c_int, close, dup2, fork, getpgid, getpgrp, pid_t, pipe, waitpid,
    STDOUT_FILENO, WNOHANG,
};

fn wait_foreground(pid: pid_t) {
    if unsafe { waitpid(pid, null_mut(), 0) } == -1 {
        panic!("waitpid last foreground process failed");
    }

    unsafe { waitpid(-getpgrp(), null_mut(), WNOHANG) };
}

#[derive(Debug)]
pub enum ExecutionError {
    Syscall(c_int),
    Precondition,
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &*self {
            ExecutionError::Syscall(error_num) => write!(
                f,
                "{}",
                unsafe { CStr::from_ptr(strerror(*error_num)) }
                    .to_string_lossy()
                    .to_owned()
            ),
            ExecutionError::Precondition => write!(f, "precondition not fulfilled"),
        }
    }
}

impl Error for ExecutionError {}

pub(crate) fn execute(cmds: &[Command], background: bool) -> Result<(), ExecutionError> {
    if cmds.len() < 1 {
        return Err(ExecutionError::Precondition);
    }

    let cmd = &cmds[0];

    let mut filedes: [c_int; 2] = [-1, -1];
    let mut in_fd: c_int = -1;
    let mut out_fd: c_int = -1;

    unsafe { pipe(filedes.as_mut_ptr()) };

    let pid = unsafe { fork() };
    match pid {
        -1 => Err(ExecutionError::Syscall(unsafe { *__errno_location() })),
        0 => {
            let result = if out_fd != -1 {
                match unsafe { dup2(out_fd, STDOUT_FILENO) } {
                    -1 => Err(ExecutionError::Syscall(unsafe { *__errno_location() })),
                    _ => match unsafe { close(out_fd) } {
                        -1 => Err(ExecutionError::Syscall(unsafe { *__errno_location() })),
                        _ => Ok(()),
                    },
                }
            } else {
                let name = CString::new(cmd.name).unwrap();
                unsafe { execvp(name.as_ptr(), null()) };
                Ok(())
            };

            result
        }
        _ => {
            wait_foreground(pid);
            Ok(())
        }
    }
}

pub(crate) fn catch_background_process(pid: pid_t) {
    let child_pgid = unsafe { getpgid(pid) };
    if (unsafe { getpgrp() } != child_pgid) && (child_pgid != -1) {
        unsafe { waitpid(pid, null_mut(), WNOHANG) };
        todo!("how to propagate error of waitpid?");
    }
}
