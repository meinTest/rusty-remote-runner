//! Tests which start the binary and call the api.

use httpc_test::Client;
use rusty_runner_api::api::{InfoResponse, RunResponse, RunStatus};
use serde_json::json;
use tokio::process::Child;

/// Starts the rusty runner binary and returns a child to abort it and a client to interact with it.
fn spawn_server() -> anyhow::Result<(Child, Client)> {
    // IANA recommended port range.
    let port = fastrand::u16(49152..65535);
    let child = tokio::process::Command::new(env!("CARGO_BIN_EXE_rusty-runner-server"))
        .kill_on_drop(true)
        .args(["--host", "127.0.0.1"])
        .args(["--port", &port.to_string()])
        .args([
            "--bash-path",
            if cfg!(windows) {
                r"C:\Program Files\Git\bin\bash.exe"
            } else {
                "bash"
            },
        ])
        .args([
            "--powershell-path",
            if cfg!(windows) { "powershell" } else { "pwsh" },
        ])
        .spawn()
        .expect("Couldn't spawn server");
    let hc = httpc_test::new_client(format!("http://localhost:{port}"))?;
    Ok((child, hc))
}

#[tokio::test(flavor = "current_thread")]
async fn info() -> anyhow::Result<()> {
    let (mut child, hc) = spawn_server()?;

    let info = hc.do_get("/api/info").await?;
    info.print().await?;
    let info: InfoResponse = info.json_body_as()?;
    assert_eq!(info.api_version, rusty_runner_api::api::VERSION);

    child.kill().await.expect("Couldn't kill server");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn pwd_command() -> anyhow::Result<()> {
    let (mut child, hc) = spawn_server()?;

    // The pwd command actually isn't found...
    let response = hc
        .do_post(
            "/api/run",
            json!({
                "command": "pwd",
                "arguments": [],
                "return_stdout": true,
            }),
        )
        .await?;
    response.print().await?;
    let _json: RunResponse = response.json_body_as()?;
    // Cannot really assert anything here.

    child.kill().await.expect("Couldn't kill server");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn bash_echo() -> anyhow::Result<()> {
    let (mut child, hc) = spawn_server()?;

    let response = hc
        .do_post(
            "/api/runscript?interpreter=bash&return_stdout=true",
            "echo 'HIIII'",
        )
        .await?;
    response.print().await?;
    let response = response.json_body_as::<RunResponse>()?;
    let RunStatus::Completed { stdout, .. } = response.status else {
        panic!("Couldn't execute echo");
    };
    assert_eq!(
        "HIIII\n",
        &String::from_utf8(stdout.expect("Was configured to return stdout"))
            .expect("is valid utf8")
    );

    child.kill().await.expect("Couldn't kill server");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn bash_cd() -> anyhow::Result<()> {
    let (mut child, hc) = spawn_server()?;

    let response = hc
        .do_post(
            "/api/runscript?interpreter=bash&return_stdout=true",
            r#"
            cd "/bin"
            pwd
            "#,
        )
        .await?;
    response.print().await?;
    let response = response.json_body_as::<RunResponse>()?;
    let RunStatus::Completed { stdout, .. } = response.status else {
        panic!("Couldn't execute echo");
    };
    let output =
        String::from_utf8(stdout.expect("Was configured to return stdout")).expect("is valid utf8");
    assert!(output.contains("/bin"));

    child.kill().await.expect("Couldn't kill server");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn powershell_echo() -> anyhow::Result<()> {
    let (mut child, hc) = spawn_server()?;

    let response = hc
        .do_post(
            "/api/runscript?interpreter=powershell&return_stdout=true",
            "echo 'HIIII'",
        )
        .await?;
    response.print().await?;
    let response = response.json_body_as::<RunResponse>()?;
    let RunStatus::Completed { stdout, .. } = response.status else {
        panic!("Couldn't execute echo");
    };
    assert_eq!(
        "HIIII\r\n",
        &String::from_utf8(stdout.expect("Was configured to return stdout"))
            .expect("is valid utf8")
    );

    child.kill().await.expect("Couldn't kill server");
    Ok(())
}

#[cfg(windows)]
#[tokio::test(flavor = "current_thread")]
async fn powershell_cd() -> anyhow::Result<()> {
    let (mut child, hc) = spawn_server()?;

    let response = hc
        .do_post(
            "/api/runscript?interpreter=powershell&return_stdout=true",
            r#"
            cd "C:\Program Files"
            pwd
            "#,
        )
        .await?;
    response.print().await?;
    let response = response.json_body_as::<RunResponse>()?;
    let RunStatus::Completed { stdout, .. } = response.status else {
        panic!("Couldn't execute echo");
    };
    let output =
        String::from_utf8(stdout.expect("Was configured to return stdout")).expect("is valid utf8");
    assert!(output.contains(r"C:\Program Files"));

    child.kill().await.expect("Couldn't kill server");
    Ok(())
}

#[cfg(windows)]
#[tokio::test(flavor = "current_thread")]
async fn cmd_echo() -> anyhow::Result<()> {
    let (mut child, hc) = spawn_server()?;

    let response = hc
        .do_post(
            "/api/runscript?interpreter=cmd&return_stdout=true",
            "@echo 'HIIII'",
        )
        .await?;
    response.print().await?;
    let response = response.json_body_as::<RunResponse>()?;
    let RunStatus::Completed { stdout, .. } = response.status else {
        panic!("Couldn't execute echo");
    };
    assert_eq!(
        "'HIIII'\r\n",
        &String::from_utf8(stdout.expect("Was configured to return stdout"))
            .expect("is valid utf8")
    );

    child.kill().await.expect("Couldn't kill server");
    Ok(())
}
