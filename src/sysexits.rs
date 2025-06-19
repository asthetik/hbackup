//! legacy exit status codes for system programs.
//! reference: [SYSEXITS](https://man.freebsd.org/cgi/man.cgi?query=sysexits&apropos=0&sektion=0&manpath=FreeBSD+11.2-stable&arch=default&format=html)

/// value: 2 <br>
/// Misuse of shell builtins (according to Bash documentation)
pub const EX_KEYWORD: i32 = 2;

/// value: 64 <br>
/// The command was used incorrectly, e.g., with the wrong number of arguments, a bad flag, a bad syntax in a parameter, etc.
pub const EX_USAGE: i32 = 64;

/// value: 65 <br>
/// The input data was incorrect in some way. This should only be used for user’s data and not system files.
pub const EX_DATAERR: i32 = 65;

/// value: 66 <br>
/// An input file (not a system file) did not exist or was not readable. This could also include errors like “No message” to a mailer (if it cared to catch it).
pub const EX_NOINPUT: i32 = 66;

/// value: 67 <br>
/// The user specified did not exist. This might be used for mail addresses or remote logins.
pub const EX_NOUSER: i32 = 67;

/// value: 68 <br>
/// The host specified did not exist. This is used in mail addresses or network requests.
pub const EX_NOHOST: i32 = 68;

/// value: 69 <br>
/// A service is unavailable. This can occur if a support program or file does not exist. This can also be used as a catchall message when something you wanted to do doesn’t work, but you don’t know why.
pub const EX_UNAVAILABLE: i32 = 69;

/// value: 70 <br>
/// An internal software error has been detected. This should be limited to non-operating system related errors as possible.
pub const EX_SOFTWARE: i32 = 70;

/// value: 71 <br>
/// An operating system error has been detected. This is intended to be used for such things as “cannot fork”, “cannot create pipe”, or the like. It includes things like getuid returning a user that does not exist in the passwd file.
pub const EX_OSERR: i32 = 71;

/// value: 72 <br>
/// Some system file (e.g., /etc/passwd, /var/run/utmp, etc.) does not exist, cannot be opened, or has some sort of error (e.g., syntax error).
pub const EX_OSFILE: i32 = 72;

/// value: 73 <br>
/// A (user specified) output file cannot be created.
pub const EX_CANTCREAT: i32 = 73;

/// value: 74 <br>
/// An error occurred while doing I/O on some file.
pub const EX_IOERR: i32 = 74;

/// value: 75 <br>
/// Temporary failure, indicating something that is not really an error. In sendmail, this means that a mailer (e.g.) could not create a connection, and the request should be reattempted later.
pub const EX_TEMPFAIL: i32 = 75;

/// value: 76 <br>
/// The remote system returned something that was “not possible” during a protocol exchange.
pub const EX_PROTOCOL: i32 = 76;

/// value: 77 <br>
/// You did not have sufficient permission to perform the operation. This is not intended for file system problems, which should use NOINPUT or CANTCREAT, but rather for higher level permissions.
pub const EX_NOPERM: i32 = 77;

/// value: 78 <br>
/// Something was found in an unconfigured or misconfigured state.
pub const EX_CONFIG: i32 = 78;
