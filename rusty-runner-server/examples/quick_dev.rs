use serde_json::{json, Value};
use rusty_runner_api::api::{RunRequest, RunResponse, RunStatus};

const URL: &str = "http://localhost:8000";

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let hc = httpc_test::new_client(URL)?;

    hc.do_get("/api/info").await?.print().await?;

    let response = hc.do_post("/api/run",pwd()).await?;
    response.print().await?;
    let json: RunResponse = serde_json::from_value(response.json_body()?)?;
    if let RunStatus::Completed { std_out_and_err: Some((out, _)), ..} = json.status {
        println!("PWD: {}", String::from_utf8(out)?);
    }

    Ok(())
}

fn pwd() -> Value {
    json!({
        "command": "pwd",
        "arguments": [],
        "return_logs": true,
    })
}