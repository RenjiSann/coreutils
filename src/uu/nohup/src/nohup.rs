// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) execvp SIGHUP cproc vprocmgr cstrs homeout

use clap::{Arg, ArgAction, Command};
use libc::{SIG_IGN, SIGHUP};
use libc::{c_char, dup2, execvp, signal};
use std::env;
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{Error, IsTerminal};
use std::os::unix::prelude::*;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{UClapError, UError, UResult, set_exit_code};
use uucore::translate;
use uucore::{format_usage, show_error};

static NOHUP_OUT: &str = "nohup.out";
// exit codes that match the GNU implementation
static EXIT_CANCELED: i32 = 125;
static EXIT_CANNOT_INVOKE: i32 = 126;
static EXIT_ENOENT: i32 = 127;
static POSIX_NOHUP_FAILURE: i32 = 127;

mod options {
    pub const CMD: &str = "cmd";
}

#[derive(Debug, Error)]
enum NohupError {
    #[error("{}", translate!("nohup-error-cannot-detach"))]
    CannotDetach,

    #[error("{}", translate!("nohup-error-cannot-replace", "name" => (*_0), "err" => _1))]
    CannotReplace(&'static str, #[source] Error),

    #[error("{}", translate!("nohup-error-open-failed", "path" => NOHUP_OUT.quote(), "err" => _1))]
    OpenFailed(i32, #[source] Error),

    #[error("{}", translate!("nohup-error-open-failed-both", "first_path" => NOHUP_OUT.quote(), "first_err" => _1, "second_path" => _2.quote(), "second_err" => _3))]
    OpenFailed2(i32, #[source] Error, String, Error),
}

impl UError for NohupError {
    fn code(&self) -> i32 {
        match self {
            Self::OpenFailed(code, _) | Self::OpenFailed2(code, _, _, _) => *code,
            _ => 2,
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args).with_exit_code(125)?;

    replace_fds()?;

    unsafe { signal(SIGHUP, SIG_IGN) };

    if unsafe { !_vprocmgr_detach_from_console(0).is_null() } {
        return Err(NohupError::CannotDetach.into());
    }

    let cstrs: Vec<CString> = matches
        .get_many::<String>(options::CMD)
        .unwrap()
        .map(|x| CString::new(x.as_bytes()).unwrap())
        .collect();
    let mut args: Vec<*const c_char> = cstrs.iter().map(|s| s.as_ptr()).collect();
    args.push(std::ptr::null());

    let ret = unsafe { execvp(args[0], args.as_mut_ptr()) };
    match ret {
        libc::ENOENT => set_exit_code(EXIT_ENOENT),
        _ => set_exit_code(EXIT_CANNOT_INVOKE),
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(translate!("nohup-about"))
        .after_help(translate!("nohup-after-help"))
        .override_usage(format_usage(&translate!("nohup-usage")))
        .arg(
            Arg::new(options::CMD)
                .hide(true)
                .required(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::CommandName),
        )
        .trailing_var_arg(true)
        .infer_long_args(true)
}

fn replace_fds() -> UResult<()> {
    if std::io::stdin().is_terminal() {
        let new_stdin = File::open(Path::new("/dev/null"))
            .map_err(|e| NohupError::CannotReplace("STDIN", e))?;
        if unsafe { dup2(new_stdin.as_raw_fd(), 0) } != 0 {
            return Err(NohupError::CannotReplace("STDIN", Error::last_os_error()).into());
        }
    }

    if std::io::stdout().is_terminal() {
        let new_stdout = find_stdout()?;
        let fd = new_stdout.as_raw_fd();

        if unsafe { dup2(fd, 1) } != 1 {
            return Err(NohupError::CannotReplace("STDOUT", Error::last_os_error()).into());
        }
    }

    if std::io::stderr().is_terminal() && unsafe { dup2(1, 2) } != 2 {
        return Err(NohupError::CannotReplace("STDERR", Error::last_os_error()).into());
    }
    Ok(())
}

fn find_stdout() -> UResult<File> {
    let internal_failure_code = match env::var("POSIXLY_CORRECT") {
        Ok(_) => POSIX_NOHUP_FAILURE,
        Err(_) => EXIT_CANCELED,
    };

    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(Path::new(NOHUP_OUT))
    {
        Ok(t) => {
            show_error!(
                "{}",
                translate!("nohup-ignoring-input-appending-output", "path" => NOHUP_OUT.quote())
            );
            Ok(t)
        }
        Err(e1) => {
            let home = match env::var("HOME") {
                Err(_) => return Err(NohupError::OpenFailed(internal_failure_code, e1).into()),
                Ok(h) => h,
            };
            let mut homeout = PathBuf::from(home);
            homeout.push(NOHUP_OUT);
            let homeout_str = homeout.to_str().unwrap();
            match OpenOptions::new().create(true).append(true).open(&homeout) {
                Ok(t) => {
                    show_error!(
                        "{}",
                        translate!("nohup-ignoring-input-appending-output", "path" => homeout_str.quote())
                    );
                    Ok(t)
                }
                Err(e2) => Err(NohupError::OpenFailed2(
                    internal_failure_code,
                    e1,
                    homeout_str.to_string(),
                    e2,
                )
                .into()),
            }
        }
    }
}

#[cfg(target_vendor = "apple")]
unsafe extern "C" {
    fn _vprocmgr_detach_from_console(flags: u32) -> *const libc::c_int;
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "openbsd"
))]
/// # Safety
/// This function is unsafe because it dereferences a raw pointer.
unsafe fn _vprocmgr_detach_from_console(_: u32) -> *const libc::c_int {
    std::ptr::null()
}
