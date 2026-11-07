// Copyright (c) 2026, Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Arrow-key menu for a terminal. Pipes keep the line prompt in `labs`.

use std::io::{
    self,
    Write,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKey {
    Up,
    Down,
    Accept,
    Cancel,
    Interrupt,
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuStep {
    Move(usize),
    Choose(usize),
    Back,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Chosen(usize),
    Back,
    Quit,
}

#[derive(Debug)]
pub enum MenuFail {
    NotATerminal,
    Io(io::Error),
}

pub fn decode_key(bytes: &[u8]) -> MenuKey {
    match bytes {
        [b'\r'] | [b'\n'] => MenuKey::Accept,
        [b'q'] | [b'Q'] => MenuKey::Cancel,
        [0x03] => MenuKey::Interrupt,
        [0x1b, b'[', b'A'] | [0x1b, b'O', b'A'] => MenuKey::Up,
        [0x1b, b'[', b'B'] | [0x1b, b'O', b'B'] => MenuKey::Down,
        [0x1b] => MenuKey::Cancel,
        _ => MenuKey::Ignore,
    }
}

pub fn step(index: usize, len: usize, key: MenuKey) -> MenuStep {
    if len == 0 {
        return MenuStep::Back;
    }
    match key {
        MenuKey::Up => MenuStep::Move(if index == 0 { len - 1 } else { index - 1 }),
        MenuKey::Down => MenuStep::Move(if index + 1 == len { 0 } else { index + 1 }),
        MenuKey::Accept => MenuStep::Choose(index),
        MenuKey::Cancel => MenuStep::Back,
        MenuKey::Interrupt => MenuStep::Quit,
        MenuKey::Ignore => MenuStep::Move(index),
    }
}

pub fn render_menu(items: &[impl AsRef<str>], index: usize) -> String {
    let mut text = String::new();
    for (i, item) in items.iter().enumerate() {
        let mark = if i == index { '>' } else { ' ' };
        text.push('\r');
        text.push_str("\u{1b}[2K");
        text.push(mark);
        text.push(' ');
        text.push_str(item.as_ref());
        text.push('\n');
    }
    text
}

pub fn raw_mode_works() -> bool {
    #[cfg(unix)]
    {
        TermGuard::enter().is_ok()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

pub fn select_index(out: &mut impl Write, items: &[impl AsRef<str>]) -> Result<Choice, MenuFail> {
    if items.is_empty() {
        return Ok(Choice::Back);
    }
    #[cfg(not(unix))]
    {
        let _ = (out, items);
        return Err(MenuFail::NotATerminal);
    }
    #[cfg(unix)]
    {
        let _raw = TermGuard::enter()?;
        let mut index = 0usize;
        draw(out, items, index, true)?;
        loop {
            match step(index, items.len(), read_menu_key()?) {
                MenuStep::Move(next) => {
                    index = next;
                    draw(out, items, index, false)?;
                }
                MenuStep::Choose(chosen) => return Ok(Choice::Chosen(chosen)),
                MenuStep::Back => return Ok(Choice::Back),
                MenuStep::Quit => return Ok(Choice::Quit),
            }
        }
    }
}

#[cfg(unix)]
fn draw(out: &mut impl Write, items: &[impl AsRef<str>], index: usize, first: bool) -> Result<(), MenuFail> {
    if !first {
        write!(out, "\u{1b}[{}A", items.len()).map_err(MenuFail::Io)?;
    }
    write!(out, "{}", render_menu(items, index)).map_err(MenuFail::Io)?;
    out.flush().map_err(MenuFail::Io)
}

#[cfg(unix)]
fn read_menu_key() -> Result<MenuKey, MenuFail> {
    let first = read_byte()?;
    if first != 0x1b {
        return Ok(decode_key(&[first]));
    }
    let mut seq = vec![0x1b];
    if poll_in(80)? {
        let second = read_byte()?;
        seq.push(second);
        if matches!(second, b'[' | b'O') && poll_in(80)? {
            seq.push(read_byte()?);
        }
    }
    Ok(decode_key(&seq))
}

#[cfg(unix)]
fn read_byte() -> Result<u8, MenuFail> {
    let mut buf = [0u8; 1];
    loop {
        // SAFETY: one-byte read from stdin. The pointer is the local buffer.
        let n = unsafe { libc::read(libc::STDIN_FILENO, buf.as_mut_ptr().cast(), 1) };
        if n == 1 {
            return Ok(buf[0]);
        }
        if n == 0 {
            return Err(MenuFail::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "stdin closed",
            )));
        }
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::Interrupted {
            continue;
        }
        return Err(MenuFail::Io(err));
    }
}

#[cfg(unix)]
fn poll_in(timeout_ms: i32) -> Result<bool, MenuFail> {
    let mut pollfd = libc::pollfd {
        fd: libc::STDIN_FILENO,
        events: libc::POLLIN,
        revents: 0,
    };
    loop {
        // SAFETY: pollfd points at one local struct. nfds is 1.
        let rc = unsafe { libc::poll(&mut pollfd, 1, timeout_ms) };
        if rc >= 0 {
            return Ok(rc > 0);
        }
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::Interrupted {
            continue;
        }
        return Err(MenuFail::Io(err));
    }
}

#[cfg(unix)]
struct TermGuard {
    orig: libc::termios,
}

#[cfg(unix)]
impl TermGuard {
    fn enter() -> Result<Self, MenuFail> {
        let mut slot = std::mem::MaybeUninit::<libc::termios>::uninit();
        // SAFETY: tcgetattr writes a termios through this pointer. On success the slot is initialized.
        let rc = unsafe { libc::tcgetattr(libc::STDIN_FILENO, slot.as_mut_ptr()) };
        if rc != 0 {
            return Err(MenuFail::NotATerminal);
        }
        // SAFETY: tcgetattr returned 0, so it initialized the struct.
        let orig = unsafe { slot.assume_init() };
        let mut raw = orig;
        // Leave output processing on so `\n` still returns the cursor to column 0.
        raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
        raw.c_cc[libc::VMIN] = 1;
        raw.c_cc[libc::VTIME] = 0;
        // SAFETY: raw is a filled termios. TCSANOW applies it to stdin.
        let rc = unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw) };
        if rc != 0 {
            return Err(MenuFail::Io(io::Error::last_os_error()));
        }
        let guard = Self { orig };
        let mut out = io::stdout();
        out.write_all(b"\x1b[?25l").map_err(MenuFail::Io)?;
        out.flush().map_err(MenuFail::Io)?;
        Ok(guard)
    }
}

#[cfg(unix)]
impl Drop for TermGuard {
    fn drop(&mut self) {
        let mut out = io::stdout();
        let _ = out.write_all(b"\x1b[?25h");
        let _ = out.flush();
        // SAFETY: orig is the termios saved before raw mode.
        unsafe {
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.orig);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_bytes_move_and_enter_accepts() {
        assert_eq!(decode_key(b"\x1b[A"), MenuKey::Up);
        assert_eq!(decode_key(b"\x1b[B"), MenuKey::Down);
        assert_eq!(decode_key(b"\x1bOA"), MenuKey::Up);
        assert_eq!(decode_key(b"\x1bOB"), MenuKey::Down);
        assert_eq!(decode_key(b"\r"), MenuKey::Accept);
        assert_eq!(decode_key(b"\n"), MenuKey::Accept);
        assert_eq!(decode_key(b"q"), MenuKey::Cancel);
        assert_eq!(decode_key(b"\x1b"), MenuKey::Cancel);
        assert_eq!(decode_key(&[0x03]), MenuKey::Interrupt);
        assert_eq!(decode_key(b"x"), MenuKey::Ignore);

        assert_eq!(step(0, 4, MenuKey::Up), MenuStep::Move(3));
        assert_eq!(step(3, 4, MenuKey::Down), MenuStep::Move(0));
        assert_eq!(step(1, 4, MenuKey::Down), MenuStep::Move(2));
        assert_eq!(step(2, 4, MenuKey::Accept), MenuStep::Choose(2));
        assert_eq!(step(2, 4, MenuKey::Cancel), MenuStep::Back);
        assert_eq!(step(2, 4, MenuKey::Interrupt), MenuStep::Quit);
    }

    #[test]
    fn render_marks_the_selected_row() {
        let text = render_menu(&["new", "edit", "delete", "quit"], 1);
        assert!(text.contains("> edit\n"), "{text}");
        assert!(text.contains("  new\n"), "{text}");
        assert!(!text.contains("> new"), "{text}");
    }
}
