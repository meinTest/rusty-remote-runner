use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Used for constructing a `tokio::process::Command`.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunRequest {
    pub command: String,
    /// The arguments as passed to `tokio::process::Command::args`
    ///
    /// # Warning
    /// [Raw args](https://doc.rust-lang.org/stable/std/os/windows/process/trait.CommandExt.html#tymethod.raw_arg)
    /// are not supported.
    /// Avoid `cmd.exe /C`!
    pub arguments: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunScriptQuery {
    #[serde(default)]
    pub interpreter: RunScriptInterpreter,
}

/// The interpreter that the script will be called with.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum RunScriptInterpreter {
    #[serde(rename = "bash")]
    Bash,
    #[serde(rename = "cmd")]
    Cmd,
    #[default]
    Native,
}

impl RunScriptInterpreter {
    pub fn as_extension(&self) -> &'static str {
        match self {
            RunScriptInterpreter::Bash => "sh",
            RunScriptInterpreter::Cmd => "bat",
            #[cfg(windows)]
            RunScriptInterpreter::Native => "bat",
            #[cfg(unix)]
            RunScriptInterpreter::Native => "sh",
        }
    }
}

/// Describes the json response format for `/api/run` and `/api/runscript`.
///
/// # Serialized Example
/// ```
/// # let ser = r#"
/// {
///     "id": "db98508d-97b1-4e2e-bb08-233de6755a8d",
///     "status": "Failure",
///     "reason": "Not supported"
/// }
/// # "#;
/// # let deser: rusty_remote_runner::api::RunResponse
/// #    = serde_json::from_str(ser).expect("failed parsing");
/// # assert!(matches!(deser.status, rusty_remote_runner::api::RunStatus::Failure(_)));
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct RunResponse {
    pub id: Uuid,
    #[serde(flatten)]
    pub status: RunStatus,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum RunStatus {
    Failure(FailureInfo),
    //Pending(),
    Completed(CompletionInfo),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FailureInfo {
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionInfo {
    // Todo: do I even want to return anything?
    /// Exit code of the command or -1001 if terminated by a signal
    pub exit_code: i32,
    // Todo: time taken
}
