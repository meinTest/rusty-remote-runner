//! This crate serves purely as an rest api abstraction for a remote script execution server.
//! Additionally there is a canonical server implementation in the same repository.
//!
//! This can be used as a ssh replacement, without having to deal with
//! command line escaping or default shells.
//!
//! ## Usage
//! For the complete usage, see the serde structs in [`api`].
//! * `GET /api/info` returns an informative [`api::InfoResponse`] object.
//! * `POST /api/run` runs a command analogous to [`std::process::Command`].
//! * `POST /api/runscript` runs the body with a given interpreter.
//! * `GET /api/file/{path}` fetches a file from the servers working directory.
//!
//! ## Working with files
//! The working directory of the executed commands is implementation defined,
//! but the same for all methods and constant over the lifetime of the server.
//! The path for file fetching is also a relative path in this directory.
//!
//! Best use a randomly named subdirectory in the current folder for your file operations.
//! E.g. `./task-9ae4ef2b9d13/your-file`
//!
//! ## Long running jobs
//! Using `reqwest` and `axum` does not impose an significant timeout on the http calls.
//! Therefore currently the calls will just wait until the command terminates and return then.
//! *Make sure your commands always terminate* in order to not lock up valuable resources.
//!
//! In a future version of the api, a [`RunStatus::Pending`](api::RunStatus) variant
//! and a status poll endpoint might be added.
//!
//! ## Security
//! The api does not include any security measures, this is *remote execution as a service!*.
//! Make sure it is only reachable from trusted hosts. E.g. by means of ssh port forwarding.

pub mod api;
