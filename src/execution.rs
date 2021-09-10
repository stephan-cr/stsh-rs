use std::error::Error;
use std::ffi::{CStr, CString};
use std::fmt::{self, Display, Formatter};
use std::ptr::{null, null_mut};

use crate::parser::Command;

use libc::{
    __errno_location, c_char, c_int, close, dup2, execvp, fork, getpgid, getpgrp, open, pid_t,
    pipe, strerror, waitpid, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY, STDOUT_FILENO, S_IRUSR, S_IWUSR,
    WNOHANG,
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
    if cmds.is_empty() {
        return Err(ExecutionError::Precondition);
    }

    let cmd = &cmds[0];

    let mut filedes: [c_int; 2] = [-1, -1];
    let mut in_fd: c_int = -1;
    let mut out_fd: c_int = -1;

    if let Some(filename) = cmd.input_file {
        let filename = CString::new(filename).unwrap();
        in_fd = unsafe { open(filename.as_ptr(), O_RDONLY) };
    }

    if let Some(filename) = cmd.output_file {
        let filename = CString::new(filename).unwrap();
        out_fd = unsafe {
            open(
                filename.as_ptr(),
                O_CREAT | O_WRONLY | O_TRUNC,
                S_IRUSR | S_IWUSR,
            )
        };
    }

    unsafe { pipe(filedes.as_mut_ptr()) };

    let pid = unsafe { fork() };
    match pid {
        -1 => Err(ExecutionError::Syscall(unsafe { *__errno_location() })),
        0 => {
            // child process
            if out_fd != -1 {
                match unsafe { dup2(out_fd, STDOUT_FILENO) } {
                    -1 => Err(ExecutionError::Syscall(unsafe { *__errno_location() })),
                    _ => match unsafe { close(out_fd) } {
                        -1 => Err(ExecutionError::Syscall(unsafe { *__errno_location() })),
                        _ => Ok(()),
                    },
                }?
            }

            let name = CString::new(cmd.name).unwrap();
            let parameters: Vec<CString> = cmd
                .parameters
                .iter()
                .map(|x| CString::new(*x).unwrap())
                .collect();
            let mut argv: Vec<*const c_char> = parameters.iter().map(|x| x.as_ptr()).collect();
            argv.insert(0, name.as_ptr());
            argv.push(null());
            unsafe { execvp(name.as_ptr(), argv.as_ptr()) };
            Ok(())
        }
        _ => {
            // parent process
            wait_foreground(pid);

            if out_fd != -1 {
                unsafe { close(out_fd) };
            }

            if in_fd != -1 {
                unsafe { close(in_fd) };
            }

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
