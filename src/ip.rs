use tokio::process::Command;

fn ip(subcommand: &str) -> Command {
    let mut command = crate::command("ip");

    command.arg(subcommand);
    command
}

// ip addr & ip link
pub async fn add(interface: &str, address: &str) -> anyhow::Result<()> {
    ip("addr")
        .arg("add")
        .arg(address)
        .arg("dev")
        .arg(interface)
        .spawn()?
        .wait()
        .await?;

    ip("link")
        .arg("set")
        .arg(interface)
        .arg("up")
        .spawn()?
        .wait()
        .await?;

    Ok(())
}

// ip route
pub async fn route(route: &str, address: &str) -> anyhow::Result<()> {
    ip("route")
        .arg("add")
        .arg(route)
        .arg("via")
        .arg(address)
        .spawn()?
        .wait()
        .await?;

    Ok(())
}
