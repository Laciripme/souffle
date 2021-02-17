use selaphiel::{mount, unistd};
use tokio::task;

pub const ROOT_PATH: &[u8] = b"/\0";
pub const DEVTMPFS_PATH: &[u8] = b"/dev\0";
pub const DEVPTS_PATH: &[u8] = b"/dev/pts\0";
pub const PROC_PATH: &[u8] = b"/proc\0";
pub const SYSFS_PATH: &[u8] = b"/sys\0";

pub const DEVTMPFS_TYPE: &[u8] = b"devtmpfs\0";
pub const DEVPTS_TYPE: &[u8] = b"devpts\0";
pub const PROC_TYPE: &[u8] = b"proc\0";
pub const SYSFS_TYPE: &[u8] = b"sysfs\0";

pub const PROC_EXTRA: &[u8] = b"hidepid=invisible\0";

pub async fn root() -> anyhow::Result<()> {
    unistd::write(0, crate::MOUNT_ROOT.as_bytes());

    let task = task::spawn_blocking(move || {
        mount::mount(
            None,
            Some(ROOT_PATH),
            None,
            (libc::MS_NOATIME | libc::MS_REMOUNT) as usize,
            None,
        )
        .map_err(|_| anyhow::anyhow!("remount / failed"))
    });

    task.await??;

    Ok(())
}

pub async fn devtmpfs() -> anyhow::Result<()> {
    unistd::write(0, crate::MOUNT_DEVTMPFS.as_bytes());

    let task = task::spawn_blocking(move || {
        mount::mount(
            Some(DEVTMPFS_TYPE),
            Some(DEVTMPFS_PATH),
            Some(DEVTMPFS_TYPE),
            libc::MS_NOATIME as usize,
            None,
        )
        .map_err(|_| anyhow::anyhow!("mount /dev failed"))
    });

    task.await??;

    Ok(())
}

pub async fn devpts() -> anyhow::Result<()> {
    unistd::write(0, crate::MOUNT_DEVPTS.as_bytes());

    let task = task::spawn_blocking(move || {
        mount::mount(
            Some(DEVPTS_TYPE),
            Some(DEVPTS_PATH),
            Some(DEVPTS_TYPE),
            libc::MS_NOATIME as usize,
            None,
        )
        .map_err(|_| anyhow::anyhow!("mount /dev/pts failed"))
    });

    task.await??;

    Ok(())
}

pub async fn proc() -> anyhow::Result<()> {
    unistd::write(0, crate::MOUNT_PROC.as_bytes());

    let task = task::spawn_blocking(move || {
        mount::mount(
            Some(PROC_TYPE),
            Some(PROC_PATH),
            Some(PROC_TYPE),
            libc::MS_NOATIME as usize,
            Some(PROC_EXTRA),
        )
        .map_err(|_| anyhow::anyhow!("mount /proc failed"))
    });

    task.await??;

    Ok(())
}

pub async fn sysfs() -> anyhow::Result<()> {
    unistd::write(0, crate::MOUNT_SYSFS.as_bytes());

    let task = task::spawn_blocking(move || {
        mount::mount(
            Some(SYSFS_TYPE),
            Some(SYSFS_PATH),
            Some(SYSFS_TYPE),
            libc::MS_NOATIME as usize,
            None,
        )
        .map_err(|_| anyhow::anyhow!("mount /sys failed"))
    });

    task.await??;

    Ok(())
}
