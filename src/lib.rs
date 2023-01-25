pub mod api {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    /// Used for constructing a [`tokio::process::Command`].
    #[derive(Debug, Serialize, Deserialize)]
    pub struct RunRequest {
        pub command: String,
        pub arguments: Vec<String>,
    }

    /// Describes the json response format for `/api/run`.
    ///
    /// e.g:
    /// `{
    /// "status": "Pending",
    /// "id": "db98508d-97b1-4e2e-bb08-233de6755a8d"
    /// }`
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "status")]
    pub enum RunCommandResponse {
        Failure(FailureInfo),
        //Pending(CommandInfo),
        Completed(CompletionInfo),
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct CommandInfo {
        pub id: Uuid,
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
}
