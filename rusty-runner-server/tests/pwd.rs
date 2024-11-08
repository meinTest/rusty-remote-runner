use rusty_runner_api::api::{InfoResponse, RunResponse, RunStatus};
use serde_json::{json, Value};

const URL: &str = "http://localhost:8000";

#[tokio::test(flavor = "current_thread")]
async fn test_info() -> anyhow::Result<()> {
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_rusty-runner-server"))
        .kill_on_drop(true)
        .spawn()
        .expect("Couldn't spawn server");
    let hc = httpc_test::new_client(URL)?;

    let info = hc.do_get("/api/info").await?;
    info.print().await?;
    let info: InfoResponse = info.json_body_as()?;
    assert_eq!(info.api_version, rusty_runner_api::api::VERSION);

    child.kill().await.expect("Couldn't kill server");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn test_command() -> anyhow::Result<()> {
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_rusty-runner-server"))
        .kill_on_drop(true)
        .spawn()
        .expect("Couldn't spawn server");
    let hc = httpc_test::new_client(URL)?;

    let response = hc.do_post("/api/run", pwd_command()).await?;
    response.print().await?;
    let _json: RunResponse = response.json_body_as()?;
    // Cannot really assert anything here.

    child.kill().await.expect("Couldn't kill server");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn test_script() -> anyhow::Result<()> {
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_rusty-runner-server"))
        .kill_on_drop(true)
        .spawn()
        .expect("Couldn't spawn server");
    let hc = httpc_test::new_client(URL)?;

    let response = hc
        .do_post(
            "/api/runscript?interpreter=bash&return_stdout=true",
            "echo 'HIIII'",
        )
        .await?;
    response.print().await?;
    let response: RunResponse = response.json_body_as()?;
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

fn pwd_command() -> Value {
    json!({
        "command": "pwd",
        "arguments": [],
        "return_stderr": true,
    })
}
