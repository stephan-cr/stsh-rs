use std::error::Error;
use std::ffi::{CStr, CString};
use std::fmt::{self, Display, Formatter};
use std::process::exit;
use std::ptr::{null, null_mut};

use crate::parser::Command;

use libc::{
    __errno_location, c_char, c_int, close, dup2, execvp, fork, getpgid, getpgrp, getpid, open,
    pid_t, pipe, setpgid, strerror, waitpid, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY, STDOUT_FILENO,
    S_IRUSR, S_IWUSR, WNOHANG,
};

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

fn wait_foreground(pid: pid_t) -> Result<(), ExecutionError> {
    if unsafe { waitpid(pid, null_mut(), 0) } == -1 {
        // panic!("waitpid last foreground process failed");
        return Err(ExecutionError::Syscall(unsafe { *__errno_location() }));
    }

    unsafe { waitpid(-getpgrp(), null_mut(), WNOHANG) };

    Ok(())
}

pub(crate) fn execute(cmds: &[Command], background: bool) -> Result<(), ExecutionError> {
    if cmds.is_empty() {
        return Err(ExecutionError::Precondition);
    }

    let cmd = &cmds[0];

    let mut filedes: [c_int; 2] = [-1, -1];
    let mut in_fd: Option<c_int> = None;
    let mut out_fd: Option<c_int> = None;
    let mut pgid = 0;

    if let Some(filename) = cmd.input_file {
        let filename = CString::new(filename).unwrap();
        in_fd = Some(unsafe { open(filename.as_ptr(), O_RDONLY) });
    }

    if let Some(filename) = cmd.output_file {
        let filename = CString::new(filename).unwrap();
        out_fd = Some(unsafe {
            open(
                filename.as_ptr(),
                O_CREAT | O_WRONLY | O_TRUNC,
                S_IRUSR | S_IWUSR,
            )
        });
    }

    unsafe { pipe(filedes.as_mut_ptr()) };

    let pid = unsafe { fork() };
    match pid {
        -1 => Err(ExecutionError::Syscall(unsafe { *__errno_location() })),
        0 => {
            // child process
            if let Some(in_fd) = in_fd {
                if unsafe { dup2(in_fd, STDOUT_FILENO) } == -1 {
                    return Err(ExecutionError::Syscall(unsafe { *__errno_location() }));
                }

                if unsafe { close(in_fd) } == -1 {
                    return Err(ExecutionError::Syscall(unsafe { *__errno_location() }));
                }
            }

            if let Some(out_fd) = out_fd {
                if unsafe { dup2(out_fd, STDOUT_FILENO) } == -1 {
                    return Err(ExecutionError::Syscall(unsafe { *__errno_location() }));
                }

                if unsafe { close(out_fd) } == -1 {
                    return Err(ExecutionError::Syscall(unsafe { *__errno_location() }));
                }
            }

            if cmd.background {
                if pgid == 0 {
                    if unsafe { setpgid(getpid(), 0) } == -1 {
                        return Err(ExecutionError::Syscall(unsafe { *__errno_location() }));
                    }
                } else {
                    if unsafe { setpgid(getpid(), pgid) } == -1 {
                        return Err(ExecutionError::Syscall(unsafe { *__errno_location() }));
                    }
                }
            }

            let name = CString::new(cmd.name).unwrap();
            let parameters: Vec<CString> = cmd
                .parameters
                .iter()
                .map(|param| CString::new(*param).unwrap())
                .collect();
            let mut argv: Vec<*const c_char> =
                parameters.iter().map(|param| param.as_ptr()).collect();
            argv.insert(0, name.as_ptr());
            argv.push(null());
            if unsafe { execvp(name.as_ptr(), argv.as_ptr()) } == -1 {
                exit(unsafe { *__errno_location() });
            }

            unreachable!("execvp");
        }
        _ => {
            // parent process
            if cmd.background {
                if pgid == 0 {
                    pgid = pid;
                }
            } else {
                wait_foreground(pid)?;
            }

            if let Some(out_fd) = out_fd {
                if unsafe { close(out_fd) } == -1 {
                    return Err(ExecutionError::Syscall(unsafe { *__errno_location() }));
                }
            }

            if let Some(in_fd) = in_fd {
                if unsafe { close(in_fd) } == -1 {
                    return Err(ExecutionError::Syscall(unsafe { *__errno_location() }));
                }
            }

            Ok(())
        }
    }
}

pub(crate) fn catch_background_process(pid: pid_t) {
    let child_pgid = unsafe { getpgid(pid) };
    if (unsafe { getpgrp() } != child_pgid) && (child_pgid != -1) {
        unsafe { waitpid(pid, null_mut(), WNOHANG) };
    }

    // we have no way to propagate errors
}
