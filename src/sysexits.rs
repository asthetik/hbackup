//! Legacy exit status codes for system programs, following the BSD `sysexits.h` convention.
//!
//! These constants provide standardized exit codes for CLI applications, allowing
//! consistent error reporting and easier integration with other system tools and scripts.
//!
//! Reference: [SYSEXITS](https://man.freebsd.org/cgi/man.cgi?query=sysexits&apropos=0&sektion=0&manpath=FreeBSD+11.2-stable&arch=default&format=html)
//!
//! # Usage
//! Use these constants with `std::process::exit()` to indicate specific error conditions.
//!
//! # Example
//! ```no_run
//! use hbackup::sysexits::EX_USAGE;
//! std::process::exit(EX_USAGE);
//! ```

/// value: 2  
/// Misuse of shell builtins (according to Bash documentation)
pub const EX_KEYWORD: i32 = 2;

/// value: 64  
/// The command was used incorrectly, e.g., with the wrong number of arguments, a bad flag, a bad syntax in a parameter, etc.
pub const EX_USAGE: i32 = 64;

/// value: 65  
/// The input data was incorrect in some way. This should only be used for user’s data and not system files.
pub const EX_DATAERR: i32 = 65;

/// value: 66  
/// An input file (not a system file) did not exist or was not readable. This could also include errors like “No message” to a mailer (if it cared to catch it).
pub const EX_NOINPUT: i32 = 66;

/// value: 67  
/// The user specified did not exist. This might be used for mail addresses or remote logins.
pub const EX_NOUSER: i32 = 67;

/// value: 68  
/// The host specified did not exist. This is used in mail addresses or network requests.
pub const EX_NOHOST: i32 = 68;

/// value: 69  
/// A service is unavailable. This can occur if a support program or file does not exist. This can also be used as a catchall message when something you wanted to do doesn’t work, but you don’t know why.
pub const EX_UNAVAILABLE: i32 = 69;

/// value: 70  
/// An internal software error has been detected. This should be limited to non-operating system related errors as possible.
pub const EX_SOFTWARE: i32 = 70;

/// value: 71  
/// An operating system error has been detected. This is intended to be used for such things as “cannot fork”, “cannot create pipe”, or the like. It includes things like getuid returning a user that does not exist in the passwd file.
pub const EX_OSERR: i32 = 71;

/// value: 72  
/// Some system file (e.g., /etc/passwd, /var/run/utmp, etc.) does not exist, cannot be opened, or has some sort of error (e.g., syntax error).
pub const EX_OSFILE: i32 = 72;

/// value: 73  
/// A (user specified) output file cannot be created.
pub const EX_CANTCREAT: i32 = 73;

/// value: 74  
/// An error occurred while doing I/O on some file.
pub const EX_IOERR: i32 = 74;

/// value: 75  
/// Temporary failure, indicating something that is not really an error. In sendmail, this means that a mailer (e.g.) could not create a connection, and the request should be reattempted later.
pub const EX_TEMPFAIL: i32 = 75;

/// value: 76  
/// The remote system returned something that was “not possible” during a protocol exchange.
pub const EX_PROTOCOL: i32 = 76;

/// value: 77  
/// You did not have sufficient permission to perform the operation. This is not intended for file system problems, which should use NOINPUT or CANTCREAT, but rather for higher level permissions.
pub const EX_NOPERM: i32 = 77;

/// value: 78  
/// Something was found in an unconfigured or misconfigured state.
pub const EX_CONFIG: i32 = 78;
