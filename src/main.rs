#![feature(asm)]
#![feature(with_options)]

use const_concat::const_concat;
use selaphiel::unistd;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Command;
use tokio::sync::Notify;
use tokio::{fs, task};

pub mod ip;
pub mod mounts;
pub mod tty;

pub mod login {
    use const_concat::const_concat;

    pub const NAME: &str = "saraph";
    pub const HOME: &str = const_concat!("/", NAME);

    pub const SHELL: &str = "zsh";

    pub const USER_ID: u32 = 1;
    pub const GROUP_ID: u32 = USER_ID;
    pub const GROUP_IDS: [u32; 3] = [3, 4, 5];
}

pub const ANSI_ESC: &str = "\x1b";
pub const ANSI_CSI: &str = const_concat!(ANSI_ESC, "[");
pub const ANSI_RESET: &str = const_concat!(ANSI_CSI, "m");

macro_rules! ANSI_SGR {
    ($ground:literal, $colour:literal) => {
        const_concat::const_concat!($crate::ANSI_CSI, $ground, ";5;", $colour, "m",)
    };
}

pub const ANSI_BRIGHT_RED: &str = ANSI_SGR!("38", "9");

pub const NEWLINE: &str = "\n";
pub const TAB: &str = "  ";

pub const HEADER: &str = const_concat!(
    NEWLINE,
    TAB,
    ANSI_BRIGHT_RED,
    env!("CARGO_PKG_NAME"),
    ANSI_RESET,
    " v",
    env!("CARGO_PKG_VERSION"),
    NEWLINE,
    NEWLINE,
);

macro_rules! ACTION {
    (remount: $message:literal) => {
        const_concat::const_concat!(
            $crate::TAB,
            $crate::TAB,
            $crate::ANSI_BRIGHT_RED,
            "remount",
            $crate::ANSI_RESET,
            " ",
            $message,
            NEWLINE,
        );
    };
    (mkdir: $path:literal) => {
        const_concat::const_concat!(
            $crate::TAB,
            $crate::TAB,
            $crate::ANSI_BRIGHT_RED,
            "  mkdir",
            $crate::ANSI_RESET,
            " ",
            $path,
            NEWLINE,
        );
    };
    (mount: $type:literal -> $path:literal) => {
        const_concat::const_concat!(
            $crate::TAB,
            $crate::TAB,
            $crate::ANSI_BRIGHT_RED,
            "  mount",
            $crate::ANSI_RESET,
            " ",
            $type,
            " -> ",
            $path,
            NEWLINE,
        );
    };
    (address: $name:literal -> $address:literal) => {
        const_concat::const_concat!(
            $crate::TAB,
            $crate::TAB,
            $crate::ANSI_BRIGHT_RED,
            "    net",
            $crate::ANSI_RESET,
            " ",
            $name,
            " -> ",
            $address,
            NEWLINE,
        );
    };
    (route: $name:literal via $address:literal) => {
        const_concat::const_concat!(
            $crate::TAB,
            $crate::TAB,
            $crate::ANSI_BRIGHT_RED,
            "    net",
            $crate::ANSI_RESET,
            " ",
            $name,
            " via ",
            $address,
            NEWLINE,
        );
    };
    (ueventd: $name:literal) => {
        const_concat::const_concat!(
            $crate::TAB,
            $crate::TAB,
            $crate::ANSI_BRIGHT_RED,
            "ueventd",
            $crate::ANSI_RESET,
            " ",
            $name,
            NEWLINE,
        );
    };
}

pub const MOUNT_ROOT: &str = ACTION!(remount: "/");

pub const MOUNT_DEVTMPFS: &str = ACTION!(mount: "devtmpfs" -> "/dev");
pub const MOUNT_DEVPTS: &str = ACTION!(mount: "devpts" -> "/dev/pts");
pub const MOUNT_PROC: &str = ACTION!(mount: "proc" -> "/proc");
pub const MOUNT_SYSFS: &str = ACTION!(mount: "sysfs" -> "/sys");

pub const MKDIR_DEVTMPFS: &str = ACTION!(mkdir: "/dev");
pub const MKDIR_DEVPTS: &str = ACTION!(mkdir: "/dev/pts");
pub const MKDIR_DEVSHM: &str = ACTION!(mkdir: "/dev/shm");
pub const MKDIR_PROC: &str = ACTION!(mkdir: "/proc");
pub const MKDIR_SYSFS: &str = ACTION!(mkdir: "/sys");

pub const NET_LO: &str = ACTION!(address: "lo" -> "127.0.0.1/8");
pub const NET_ETH0: &str = ACTION!(address: "eth0" -> "192.168.20.69/24");
pub const NET_ROUTE: &str = ACTION!(route: "default" via "192.168.20.1");

pub const UEVENTD: &str = ACTION!(ueventd: "udevd");
pub const UEVENTD_POPULATE: &str = ACTION!(ueventd: "udevadm trigger");

async fn remount_root(writable: Arc<Notify>) -> anyhow::Result<()> {
    mounts::root().await;
    writable.notify_waiters();

    Ok(())
}

async fn mount_devpts() -> anyhow::Result<()> {
    let path = Path::new("/dev/pts");

    if !path.exists() {
        unistd::write(0, MKDIR_DEVPTS.as_bytes());
        fs::create_dir(&path).await;
    }

    mounts::devpts().await;

    Ok(())
}

async fn mount_devshm() -> anyhow::Result<()> {
    let path = Path::new("/dev/shm");

    if !path.exists() {
        unistd::write(0, MKDIR_DEVPTS.as_bytes());
        fs::create_dir(&path).await;
    }

    Ok(())
}

async fn mount_dev(writable: Arc<Notify>) -> anyhow::Result<()> {
    let path = Path::new("/dev");

    if !path.exists() {
        writable.notified().await;
        unistd::write(0, MKDIR_DEVTMPFS.as_bytes());
        fs::create_dir(&path).await;
    }

    mounts::devtmpfs().await;

    let task0 = tokio::spawn(mount_devpts());
    let task1 = tokio::spawn(mount_devshm());
    let results = tokio::join!(task0, task1);

    results.0??;
    results.1??;

    Ok(())
}

async fn mount_proc(writable: Arc<Notify>) -> anyhow::Result<()> {
    let path = Path::new("/proc");

    if !path.exists() {
        writable.notified().await;
        unistd::write(0, MKDIR_PROC.as_bytes());
        fs::create_dir(&path).await;
    }

    mounts::proc().await;

    Ok(())
}

async fn mount_sys(writable: Arc<Notify>) -> anyhow::Result<()> {
    let path = Path::new("/sys");

    if !path.exists() {
        writable.notified().await;
        unistd::write(0, MKDIR_PROC.as_bytes());
        fs::create_dir(&path).await;
    }

    mounts::sysfs().await;

    unistd::write(0, NET_LO.as_bytes());
    ip::add("lo", "127.0.0.1/8").await;

    unistd::write(0, NET_ETH0.as_bytes());
    ip::add("eth0", "192.168.20.69/24").await;

    unistd::write(0, NET_ROUTE.as_bytes());
    ip::route("default", "192.168.20.1").await;

    unistd::write(0, UEVENTD.as_bytes());
    command("udevd").spawn();

    unistd::write(0, UEVENTD_POPULATE.as_bytes());
    command("udevadm").arg("trigger").spawn();

    Ok(())
}

pub fn command(command: &str) -> Command {
    let mut command = Command::new(command);

    command.env("PATH", "/bin:/sbin:/usr/local/bin:/usr/local/sbin");
    command
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let start = Instant::now();

    unistd::write(0, HEADER.as_bytes());
    unistd::setsid();

    tokio::spawn(async move {
        use tokio::signal::unix::{signal, SignalKind};

        let mut stream = signal(SignalKind::child()).unwrap();

        loop {
            stream.recv().await;

            let mut status = 0;
            while unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) } > 0 {}
        }
    });

    let writable = Arc::new(Notify::const_new());
    let task0 = task::spawn(remount_root(Arc::clone(&writable)));
    let task1 = task::spawn(mount_dev(Arc::clone(&writable)));
    let task2 = task::spawn(mount_proc(Arc::clone(&writable)));
    let task3 = task::spawn(mount_sys(writable));
    let results = tokio::join!(task0, task1, task2, task3);

    results.0??;
    results.1??;
    results.2??;
    results.3??;

    println!(
        "{tab}{tab}   {red}done{reset} in {ms}ms",
        ms = start.elapsed().as_millis(),
        red = ANSI_BRIGHT_RED,
        reset = ANSI_RESET,
        tab = TAB
    );

    let task0 = tokio::spawn(tty::agetty(1));
    let task1 = tokio::spawn(tty::agetty(2));
    let results = tokio::join!(task0, task1);

    results.0??;
    results.1??;

    Ok(())
}
