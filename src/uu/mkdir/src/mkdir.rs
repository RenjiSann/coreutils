// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ugoa cmode

use clap::builder::ValueParser;
use clap::parser::ValuesRef;
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use uucore::error::FromIo;
use uucore::error::{UResult, USimpleError};
use uucore::locale::{get_message, get_message_with_args};
#[cfg(not(windows))]
use uucore::mode;
use uucore::{display::Quotable, fs::dir_strip_dot_for_creation};
use uucore::{format_usage, show_if_err};

static DEFAULT_PERM: u32 = 0o777;

mod options {
    pub const MODE: &str = "mode";
    pub const PARENTS: &str = "parents";
    pub const VERBOSE: &str = "verbose";
    pub const DIRS: &str = "dirs";
    pub const SELINUX: &str = "z";
    pub const CONTEXT: &str = "context";
}

/// Configuration for directory creation.
pub struct Config<'a> {
    /// Create parent directories as needed.
    pub recursive: bool,

    /// File permissions (octal).
    pub mode: u32,

    /// Print message for each created directory.
    pub verbose: bool,

    /// Set `SELinux` security context.
    pub set_selinux_context: bool,

    /// Specific `SELinux` context.
    pub context: Option<&'a String>,
}

#[cfg(windows)]
fn get_mode(_matches: &ArgMatches, _mode_had_minus_prefix: bool) -> Result<u32, String> {
    Ok(DEFAULT_PERM)
}

#[cfg(not(windows))]
fn get_mode(matches: &ArgMatches, mode_had_minus_prefix: bool) -> Result<u32, String> {
    // Not tested on Windows
    let mut new_mode = DEFAULT_PERM;

    if let Some(m) = matches.get_one::<String>(options::MODE) {
        for mode in m.split(',') {
            if mode.chars().any(|c| c.is_ascii_digit()) {
                new_mode = mode::parse_numeric(new_mode, m, true)?;
            } else {
                let cmode = if mode_had_minus_prefix {
                    // clap parsing is finished, now put prefix back
                    format!("-{mode}")
                } else {
                    mode.to_string()
                };
                new_mode = mode::parse_symbolic(new_mode, &cmode, mode::get_umask(), true)?;
            }
        }
        Ok(new_mode)
    } else {
        // If no mode argument is specified return the mode derived from umask
        Ok(!mode::get_umask() & 0o0777)
    }
}

#[cfg(windows)]
fn strip_minus_from_mode(_args: &mut [OsString]) -> UResult<bool> {
    Ok(false)
}

// Iterate 'args' and delete the first occurrence
// of a prefix '-' if it's associated with MODE
// e.g. "chmod -v -xw -R FILE" -> "chmod -v xw -R FILE"
#[cfg(not(windows))]
fn strip_minus_from_mode(args: &mut Vec<OsString>) -> UResult<bool> {
    for arg in args {
        if arg == "--" {
            break;
        }
        let bytes = uucore::os_str_as_bytes(arg)?;
        if let Some(b'-') = bytes.first() {
            if let Some(
                b'r' | b'w' | b'x' | b'X' | b's' | b't' | b'u' | b'g' | b'o' | b'0'..=b'7',
            ) = bytes.get(1)
            {
                *arg = uucore::os_str_from_bytes(&bytes[1..])?.into_owned();
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let mut args: Vec<OsString> = args.collect();

    // Before we can parse 'args' with clap (and previously getopts),
    // a possible MODE prefix '-' needs to be removed (e.g. "chmod -x FILE").
    let mode_had_minus_prefix = strip_minus_from_mode(&mut args)?;

    // Linux-specific options, not implemented
    // opts.optflag("Z", "context", "set SELinux security context" +
    // " of each created directory to CTX"),
    let matches = uu_app()
        .after_help(get_message("mkdir-after-help"))
        .try_get_matches_from(args)?;

    let dirs = matches
        .get_many::<OsString>(options::DIRS)
        .unwrap_or_default();
    let verbose = matches.get_flag(options::VERBOSE);
    let recursive = matches.get_flag(options::PARENTS);

    // Extract the SELinux related flags and options
    let set_selinux_context = matches.get_flag(options::SELINUX);
    let context = matches.get_one::<String>(options::CONTEXT);

    match get_mode(&matches, mode_had_minus_prefix) {
        Ok(mode) => {
            let config = Config {
                recursive,
                mode,
                verbose,
                set_selinux_context: set_selinux_context || context.is_some(),
                context,
            };
            exec(dirs, &config)
        }
        Err(f) => Err(USimpleError::new(1, f)),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(get_message("mkdir-about"))
        .override_usage(format_usage(&get_message("mkdir-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::MODE)
                .short('m')
                .long(options::MODE)
                .help(get_message("mkdir-help-mode")),
        )
        .arg(
            Arg::new(options::PARENTS)
                .short('p')
                .long(options::PARENTS)
                .help(get_message("mkdir-help-parents"))
                .overrides_with(options::PARENTS)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long(options::VERBOSE)
                .help(get_message("mkdir-help-verbose"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SELINUX)
                .short('Z')
                .help(get_message("mkdir-help-selinux"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CONTEXT)
                .long(options::CONTEXT)
                .value_name("CTX")
                .help(get_message("mkdir-help-context")),
        )
        .arg(
            Arg::new(options::DIRS)
                .action(ArgAction::Append)
                .num_args(1..)
                .required(true)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::DirPath),
        )
}

/**
 * Create the list of new directories
 */
fn exec(dirs: ValuesRef<OsString>, config: &Config) -> UResult<()> {
    for dir in dirs {
        let path_buf = PathBuf::from(dir);
        let path = path_buf.as_path();

        show_if_err!(mkdir(path, config));
    }
    Ok(())
}

/// Create directory at a given `path`.
///
/// ## Options
///
/// * `recursive` --- create parent directories for the `path`, if they do not
///   exist.
/// * `mode` --- file mode for the directories (not implemented on windows).
/// * `verbose` --- print a message for each printed directory.
///
/// ## Trailing dot
///
/// To match the GNU behavior, a path with the last directory being a single dot
/// (like `some/path/to/.`) is created (with the dot stripped).
pub fn mkdir(path: &Path, config: &Config) -> UResult<()> {
    if path.as_os_str().is_empty() {
        return Err(USimpleError::new(
            1,
            get_message("mkdir-error-empty-directory-name"),
        ));
    }
    // Special case to match GNU's behavior:
    // mkdir -p foo/. should work and just create foo/
    // std::fs::create_dir("foo/."); fails in pure Rust
    let path_buf = dir_strip_dot_for_creation(path);
    let path = path_buf.as_path();
    create_dir(path, false, config)
}

#[cfg(any(unix, target_os = "redox"))]
fn chmod(path: &Path, mode: u32) -> UResult<()> {
    use std::fs::{Permissions, set_permissions};
    use std::os::unix::fs::PermissionsExt;
    let mode = Permissions::from_mode(mode);
    set_permissions(path, mode).map_err_context(|| {
        get_message_with_args(
            "mkdir-error-cannot-set-permissions",
            HashMap::from([("path".to_string(), path.quote().to_string())]),
        )
    })
}

#[cfg(windows)]
fn chmod(_path: &Path, _mode: u32) -> UResult<()> {
    // chmod on Windows only sets the readonly flag, which isn't even honored on directories
    Ok(())
}

// Return true if the directory at `path` has been created by this call.
// `is_parent` argument is not used on windows
#[allow(unused_variables)]
fn create_dir(path: &Path, is_parent: bool, config: &Config) -> UResult<()> {
    let path_exists = path.exists();
    if path_exists && !config.recursive {
        return Err(USimpleError::new(
            1,
            get_message_with_args(
                "mkdir-error-file-exists",
                HashMap::from([("path".to_string(), path.to_string_lossy().to_string())]),
            ),
        ));
    }
    if path == Path::new("") {
        return Ok(());
    }

    if config.recursive {
        match path.parent() {
            Some(p) => create_dir(p, true, config)?,
            None => {
                USimpleError::new(1, get_message("mkdir-error-failed-to-create-tree"));
            }
        }
    }

    match std::fs::create_dir(path) {
        Ok(()) => {
            if config.verbose {
                println!(
                    "{}",
                    get_message_with_args(
                        "mkdir-verbose-created-directory",
                        HashMap::from([
                            ("util_name".to_string(), uucore::util_name().to_string()),
                            ("path".to_string(), path.quote().to_string())
                        ])
                    )
                );
            }

            #[cfg(all(unix, target_os = "linux"))]
            let new_mode = if path_exists {
                config.mode
            } else {
                // TODO: Make this macos and freebsd compatible by creating a function to get permission bits from
                // acl in extended attributes
                let acl_perm_bits = uucore::fsxattr::get_acl_perm_bits_from_xattr(path);

                if is_parent {
                    (!mode::get_umask() & 0o777) | 0o300 | acl_perm_bits
                } else {
                    config.mode | acl_perm_bits
                }
            };
            #[cfg(all(unix, not(target_os = "linux")))]
            let new_mode = if is_parent {
                (!mode::get_umask() & 0o777) | 0o300
            } else {
                config.mode
            };
            #[cfg(windows)]
            let new_mode = config.mode;

            chmod(path, new_mode)?;

            // Apply SELinux context if requested
            #[cfg(feature = "selinux")]
            if config.set_selinux_context && uucore::selinux::is_selinux_enabled() {
                if let Err(e) = uucore::selinux::set_selinux_security_context(path, config.context)
                {
                    let _ = std::fs::remove_dir(path);
                    return Err(USimpleError::new(1, e.to_string()));
                }
            }

            Ok(())
        }

        Err(_) if path.is_dir() => Ok(()),
        Err(e) => Err(e.into()),
    }
}
