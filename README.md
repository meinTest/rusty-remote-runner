# rusty-runner-api

This crate serves purely as an rest api abstraction for a remote
script execution server.

This can be used as a ssh replacement, without having to deal with
command line escaping or default shells.

### Usage
For the complete usage, see the serde structs in [`api`].
* `GET /api/info` returns an informative [`api::InfoResponse`] object.
* `POST /api/run` runs a command analogous to [`std::process::Command`].
* `POST /api/runscript` runs a script with a given interpreter.
* `GET /api/file` fetches a file from the servers file system.

### Working with files
The working directory of the executed commands is implementation defined,
but the same for all methods and constant over the lifetime of the server.
The path for file fetching is also relative to this directory.

Best use a relative randomly named subdirectory for your file operations.
E.g. `./task-9ae4ef2b9d13/your-file`

### Long running jobs
Using `reqwest` and `actix_web` does not impose and significant timeout on
the rest calls. Therefore currently the rest calls will just wait untill
the command terminates and return then.
*Make sure your commands always terminate* in order to not lock up valuable
resources.

In a future version of the api, a [`RunStatus::Pending`](api::RunStatus) variant
might be added in addition to a poll rest call.

### Security
The api does not include any security measures. Make sure it is only
reachable from trusted hosts. E.g. by means of ssh port forwarding.

License: MIT OR Apache-2.0
