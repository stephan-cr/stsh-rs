// https://github.com/vorner/signal-hook/blob/master/signal-hook-registry/src/lib.rs

use std::error::Error;
use std::ffi::CStr;
use std::fmt;
use std::mem::MaybeUninit;
use std::ptr::null_mut;

use libc::{
    __errno_location, c_int, c_void, sigaction, sigaddset, sigemptyset, sighandler_t, siginfo_t,
    sigprocmask, sigset_t, strerror,
};

use crate::execution::catch_background_process;

#[derive(Debug, PartialEq)]
pub enum SigError {
    Syscall(c_int),
}

impl fmt::Display for SigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self {
            SigError::Syscall(error_num) => write!(
                f,
                "{}",
                unsafe { CStr::from_ptr(strerror(*error_num)) }
                    .to_string_lossy()
                    .to_owned()
            ),
        }
    }
}

impl Error for SigError {}

pub(crate) extern "C" fn handler(sig: c_int, info: *mut siginfo_t, _gdata: *mut c_void) {
    if sig == libc::SIGCHLD {
        catch_background_process(unsafe { (&*info).si_pid() });
    }
}

pub(crate) fn mask_sigchld() -> Result<sigset_t, SigError> {
    let mut chld_set = unsafe { MaybeUninit::<sigset_t>::zeroed().assume_init() };

    unsafe { sigemptyset(&mut chld_set as *mut _) };
    unsafe { sigaddset(&mut chld_set as *mut _, libc::SIGCHLD) };

    match unsafe { sigprocmask(libc::SIG_BLOCK, &chld_set as *const _, null_mut()) } {
        -1 => Err(SigError::Syscall(unsafe { *__errno_location() })),
        _ => Ok(chld_set),
    }
}

pub(crate) fn unmask_sigchld(chld_set: sigset_t) -> Result<(), SigError> {
    match unsafe { sigprocmask(libc::SIG_UNBLOCK, &chld_set as *const _, null_mut()) } {
        -1 => Err(SigError::Syscall(unsafe { *__errno_location() })),
        _ => Ok(()),
    }
}

pub(crate) fn install_sighandler(
    signum: c_int,
    handler: extern "C" fn(c_int, *mut siginfo_t, *mut c_void),
) -> Result<(), SigError> {
    let sa: sigaction = sigaction {
        sa_flags: libc::SA_NOCLDSTOP | libc::SA_SIGINFO,
        sa_sigaction: handler as sighandler_t,
        sa_mask: unsafe { MaybeUninit::<sigset_t>::zeroed().assume_init() },
        sa_restorer: None,
    };

    match unsafe { libc::sigaction(signum, &sa, null_mut()) } {
        -1 => Err(SigError::Syscall(unsafe { *__errno_location() })),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use lazy_static::lazy_static;
    use libc::{c_int, c_void, siginfo_t};
    use std::mem::MaybeUninit;
    use std::ptr::{read_volatile, write_volatile};
    use std::sync::Mutex;

    #[test]
    fn test_install_sighandler() {
        assert_eq!(
            super::install_sighandler(libc::SIGCHLD, super::handler),
            Ok(())
        );
    }

    lazy_static! {
        static ref is_handler_called: Mutex<bool> = Mutex::new(false);
    }

    #[test]
    fn test_handle_signal() -> Result<(), Box<dyn std::error::Error>> {
        extern "C" fn handler(sig: c_int, _info: *mut siginfo_t, _gdata: *mut c_void) {
            if sig == libc::SIGUSR1 {
                unsafe { write_volatile(&mut (*is_handler_called.lock().unwrap()), true) };
            }
        }

        super::install_sighandler(libc::SIGUSR1, handler)?;

        let sigset = unsafe {
            let mut sigset = MaybeUninit::uninit();
            libc::sigemptyset(sigset.as_mut_ptr());
            libc::sigaddset(sigset.as_mut_ptr(), libc::SIGUSR1);
            sigset.assume_init()
        };

        // before SIGUSR1 is raised, block SIGUSR1 such it cannot be
        // raised early
        let old_sigset = unsafe {
            let mut old_sigset = MaybeUninit::uninit();
            libc::sigprocmask(libc::SIG_BLOCK, &sigset, old_sigset.as_mut_ptr());

            old_sigset.assume_init()
        };

        // raise the signal, but it has no effect, since it's block,
        // see comment before.
        unsafe {
            libc::raise(libc::SIGUSR1);
        }

        // wait for signal and set new signal mask, such that SIGUSR1
        // gets unblocked
        unsafe { libc::sigsuspend(&old_sigset) };

        assert!(unsafe { read_volatile(&(*is_handler_called.lock().unwrap())) });

        Ok(())
    }
}
