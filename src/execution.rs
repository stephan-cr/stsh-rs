use std::error::Error;
use std::ptr::null_mut;

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

pub enum ExecutionError {
    Syscall(c_int),
}

pub(crate) fn execute(background: bool) -> Result<(), ExecutionError> {
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
                Ok(())
            };

            result
        }
        _ => unimplemented!("parent"),
    }
}

pub(crate) fn catch_background_process(pid: pid_t) {
    let child_pgid = unsafe { getpgid(pid) };
    if (unsafe { getpgrp() } != child_pgid) && (child_pgid != -1) {
        unsafe { waitpid(pid, null_mut(), WNOHANG) };
        todo!("how to propagate error of waitpid?");
    }
}
