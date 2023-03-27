//! This module defines the API for this crate and its server.
//!
//! ## Working with files
//! The working directory of the executed commands is implementation defined,
//! but the same for all methods and constant over the lifetime of the server.
//! The path for file fetching is also relative to this directory.
//!
//! Best use a relative randomly named subdirectory for your file operations.
//! E.g. `./task-9ae4ef2b9d13/your-file`

use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The json-response schema for `GET /api/info`.
#[derive(Debug, Serialize, Deserialize)]
pub struct InfoResponse {
    /// The operating system type running.
    pub os_type: OsType,
    /// Any descriptive name of the runner.
    pub computer_name: String,
    /// The version of the api supported. Defined by [`VERSION`].
    pub api_version: String,
}

/// The OS type as given by `#[cfg(windows)]` and `#[cfg(unix)]`
#[derive(Debug, Serialize, Deserialize)]
pub enum OsType {
    Windows,
    Unix,
}

/// The json-body schema for `POST /api/run`.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunRequest {
    /// The command as available on the path or a path to an executable.
    pub command: String,
    /// The arguments as passed to `tokio::process::Command::args`
    ///
    /// # Warning
    /// [Raw args](https://doc.rust-lang.org/stable/std/os/windows/process/trait.CommandExt.html#tymethod.raw_arg)
    /// are not supported.
    /// Avoid `cmd.exe /C`!
    pub arguments: Vec<String>,
    /// `true` if the api should return `stdout` and `stderr`. Otherwise only
    /// the exit code is returned.
    pub return_logs: bool,
}

/// The query schema for `POST /api/runscript`.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunScriptQuery {
    /// The script in the request body will be run by the given `interpreter`.
    pub interpreter: ScriptInterpreter,
    /// `true` if the api should return `stdout` and `stderr`. Otherwise only
    /// the exit code is returned. Defaults to `false`.
    #[serde(default)]
    pub return_logs: bool,
}

/// The interpreter that the script will be called with.
///
/// Not all interpreters may be supported by any runner.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScriptInterpreter {
    Bash,
    Cmd,
    Powershell,
}

impl ScriptInterpreter {
    /// Returns the default file extension.
    pub fn as_extension(&self) -> &'static str {
        match self {
            ScriptInterpreter::Bash => "sh",
            ScriptInterpreter::Cmd => "bat",
            ScriptInterpreter::Powershell => "ps1",
        }
    }
}

/// The query schema for `GET /api/file`.
#[derive(Debug, Serialize, Deserialize)]
pub struct GetFileQuery {
    /// The path of the file to fetch. See also module documentation.
    pub path: String,
}

/// The json response format for `/api/run` and `/api/runscript`.
///
/// # Serialized Examples
/// ```
/// # let ser = r#"
/// {
///     "id": 73001,
///     "status": "Completed",
///     "exit_code": 1,
///     "time_taken": {
///         "secs": 21,
///         "nanos": 800000
///     }
/// }
/// # "#;
/// # let deser: rusty_remote_runner::api::RunResponse
/// #    = serde_json::from_str(ser).expect("failed parsing");
/// # assert!(matches!(deser.status, rusty_remote_runner::api::RunStatus::Completed { .. }));
/// # let ser = r#"
/// {
///     "id": 1234567890,
///     "status": "Failure",
///     "reason": "Not supported"
/// }
/// # "#;
/// # let deser: rusty_remote_runner::api::RunResponse
/// #    = serde_json::from_str(ser).expect("failed parsing");
/// # assert!(matches!(deser.status, rusty_remote_runner::api::RunStatus::Failure { .. }));
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct RunResponse {
    pub id: u64,
    #[serde(flatten)]
    pub status: RunStatus,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum RunStatus {
    /// Completely ran the command. The command may have succeeded of failed.
    Completed {
        /// Exit code of the command or -1001 if terminated by a signal.
        /// This may get only return the least byte.
        exit_code: i32,
        /// If `return_logs` is set, this returns a tuple of the raw `stdout` and `stderr`
        /// logs.
        std_out_and_err: Option<(Vec<u8>, Vec<u8>)>,
        /// The wall time it took to run.
        time_taken: Duration,
    },
    /// Failed to run the command due to internal reasons.
    /// Does not indicate a command that ran with a non-success exit code, but
    /// rather that the command couldn't even be started.
    Failure { reason: String },
}
