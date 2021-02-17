use crate::login;
use std::fs::File;
use std::io;
use std::io::{Error, Write};
use std::mem::MaybeUninit;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::process::Stdio;
use tokio::io::unix::AsyncFd;

pub fn tcgetattr(fd: &impl AsRawFd) -> io::Result<libc::termios> {
    let mut attr = MaybeUninit::uninit();

    if unsafe { libc::tcgetattr(fd.as_raw_fd(), attr.as_mut_ptr()) } == -1 {
        Err(Error::last_os_error())?;
    }

    Ok(unsafe { attr.assume_init() })
}

pub fn tcsetattr(fd: &impl AsRawFd, act: libc::c_int, attr: &libc::termios) -> io::Result<()> {
    if unsafe { libc::tcsetattr(fd.as_raw_fd(), act, attr as *const libc::termios) } == -1 {
        Err(Error::last_os_error())?;
    }

    Ok(())
}

fn dup(fd: &impl AsRawFd) -> io::Result<RawFd> {
    let fd = unsafe { libc::dup(fd.as_raw_fd()) };

    if fd == -1 {
        Err(Error::last_os_error())?;
    }

    Ok(fd)
}

pub async fn agetty(tty: impl AsRawFd) -> anyhow::Result<()> {
    let file = File::with_options()
        .read(true)
        .write(true)
        .open(match tty.as_raw_fd() {
            1 => "/dev/tty1",
            2 => "/dev/tty2",
            _ => return Err(anyhow::anyhow!("invalid tty")),
        })?;

    let fd = AsyncFd::new(file)?;
    let cooked = tcgetattr(&fd)?;
    let mut raw = cooked;

    unsafe { libc::cfmakeraw(&mut raw as *mut libc::termios) };

    loop {
        let mut file = unsafe { File::from_raw_fd(fd.as_raw_fd()) };

        write!(
            file,
            "{newline}{tab}{red}login{reset} {user} with {shell}{newline}",
            newline = crate::NEWLINE,
            tab = crate::TAB,
            red = crate::ANSI_BRIGHT_RED,
            reset = crate::ANSI_RESET,
            user = login::HOME,
            shell = login::SHELL,
        );

        // don't drop/close the fd
        file.into_raw_fd();

        tcsetattr(&fd, libc::TCSANOW, &raw)?;
        fd.readable().await?.clear_ready();
        tcsetattr(&fd, libc::TCSANOW, &cooked)?;

        let stdin = dup(&fd)?;
        let stdout = dup(&fd)?;
        let stderr = dup(&fd)?;

        unsafe {
            crate::command(login::SHELL)
                .current_dir(login::HOME)
                .env("HOME", login::HOME)
                .stdin(Stdio::from_raw_fd(stdin))
                .stdout(Stdio::from_raw_fd(stdout))
                .stderr(Stdio::from_raw_fd(stderr))
                .pre_exec(|| {
                    libc::setgroups(login::GROUP_IDS.len(), login::GROUP_IDS.as_ptr());
                    libc::setgid(login::USER_ID);
                    libc::setuid(login::GROUP_ID);

                    Ok(())
                })
                .spawn()?
                .wait()
                .await?;
        }
    }
}
