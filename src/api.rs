//! This module defines the API query, request body and response body
//! schema for this crate and its server by means of serde serializable
//! and deserializable rust structs.

use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The json-response schema for `GET /api/info`.
///
/// # Serialized Example
/// ```
/// # let ser = r#"
/// {
///    "os_type": "Unix",
///    "computer_name": "GLaDOS",
///    "api_version": "1.0.0"
/// }
/// # "#;
/// # let deser: rusty_runner_api::api::InfoResponse
/// #    = serde_json::from_str(ser).expect("failed parsing");
/// # assert_eq!(deser.computer_name, "GLaDOS");
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct InfoResponse {
    /// The operating system type running.
    pub os_type: OsType,
    /// Any descriptive name of the runner.
    pub computer_name: String,
    /// The version of the api supported. Defined by [`VERSION`].
    pub api_version: String,
}

/// The OS type as given by `#[cfg(windows)]` and `#[cfg(unix)]`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OsType {
    Windows,
    Unix,
}

/// The json-body schema for `POST /api/run`.
///
/// # Serialized Example
/// ```
/// # let ser = r#"
/// {
///  "command": "echo",
///  "arguments": [
///    "Hello",
///    "World"
///  ],
///  "return_logs": true
///}
/// # "#;
/// # let deser: rusty_runner_api::api::RunRequest
/// #    = serde_json::from_str(ser).expect("failed parsing");
/// # assert_eq!(deser.command, "echo");
/// ```
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
///
/// # Serialized Example
/// ```
/// # let ser = r#"
/// interpreter=bash&return_logs=false
/// # "#;
/// # let deser: rusty_runner_api::api::RunScriptQuery
/// #    = serde_urlencoded::from_str(ser.trim()).expect("failed parsing");
/// # assert!(matches!(deser.interpreter, rusty_runner_api::api::ScriptInterpreter::Bash));
/// ```
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
    #[must_use]
    pub fn as_extension(&self) -> &'static str {
        match self {
            ScriptInterpreter::Bash => "sh",
            ScriptInterpreter::Cmd => "bat",
            ScriptInterpreter::Powershell => "ps1",
        }
    }
}

/// The json response format for `/api/run` and `/api/runscript`.
///
/// # Serialized Examples
/// A completed command:
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
/// # let deser: rusty_runner_api::api::RunResponse
/// #    = serde_json::from_str(ser).expect("failed parsing");
/// # assert!(matches!(deser.status, rusty_runner_api::api::RunStatus::Completed { .. }));
/// ```
/// A command that could not be executed:
/// ```
/// # let ser = r#"
/// {
///     "id": 1234567890,
///     "status": "Failure",
///     "reason": "Not supported"
/// }
/// # "#;
/// # let deser: rusty_runner_api::api::RunResponse
/// #    = serde_json::from_str(ser).expect("failed parsing");
/// # assert!(matches!(deser.status, rusty_runner_api::api::RunStatus::Failure { .. }));
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct RunResponse {
    pub id: u64,
    #[serde(flatten)]
    pub status: RunStatus,
}

/// The outcome of a command.
///
/// If the command could be started, then this is a [`Completed`](RunStatus::Completed)
/// even if the command itself exited non-successfully.
/// Otherwise this is [`Failure`](RunStatus::Failure).
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
        // TODO: It would be MUCH better for consumer clients if this was split into 2 fields.
        std_out_and_err: Option<(Vec<u8>, Vec<u8>)>,
        /// The wall time it took to run.
        time_taken: Duration,
    },
    /// Failed to run the command due to internal reasons.
    /// Does not indicate a command that ran with a non-success exit code, but
    /// rather that the command couldn't even be started.
    Failure { reason: String },
}
