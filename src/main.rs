use std::fs::OpenOptions;
use std::io::{stdin, stdout, BufWriter, Write};
use std::{env, io};
use termion::event::{Event, Key, MouseButton, MouseEvent};
use termion::input::{MouseTerminal, TermRead};
use termion::raw::IntoRawMode;

use crate::cell::{Format, Width};
use crate::data_store::DataStore;
use crate::editor::*;

mod data_store;
#[macro_use]
mod terminal;
mod cell;
mod disasm;
mod editor;
mod util;

fn handle_key<W: Write>(key: Key, editor: &mut Editor<W>) {
    match key {
        Key::Esc => editor.set_mode(EditorMode::Normal),
        Key::Char(c) if editor.is_cmd() => editor.type_cmd(c),
        Key::Backspace if editor.is_cmd() => editor.type_cmd('\x08'),
        Key::Char(c) if editor.is_ins() => editor.insert(c),
        Key::Char(':') => editor.set_mode(EditorMode::Command),
        Key::Char('i') => editor.set_mode(EditorMode::Insert),
        Key::Right | Key::Char('l') => editor.move_cursor_next(),
        Key::Left | Key::Char('h') => editor.move_cursor_prev(),
        Key::Down | Key::Char('j') => editor.move_cursor_y(1),
        Key::Up | Key::Char('k') => editor.move_cursor_y(-1),
        Key::PageDown => editor.move_cursor_y(editor.height as isize),
        Key::PageUp => editor.move_cursor_y(-(editor.height as isize)),
        Key::Home => editor.set_cursor(0, 0),
        Key::End => editor.set_cursor_end(),
        Key::Char('p') => editor.follow_pointer(),
        Key::Char('f') => editor.switch_format(false),
        Key::Char('F') => editor.switch_format(true),
        Key::Char('x') => editor.set_format(Format::Hex),
        Key::Char('d') => editor.set_format(Format::SDec),
        Key::Char('u') => editor.set_format(Format::UDec),
        Key::Char('t') => editor.set_format(Format::Bin),
        Key::Char('c') => editor.set_format(Format::Char),
        Key::Char('s') => editor.format_string(),
        Key::Char('e') => editor.switch_byte_order(),
        Key::Char('+') => editor.inc_width(),
        Key::Char('-') => editor.dec_width(),
        Key::Char('b') => editor.set_width(Width::Byte8),
        Key::Char('w') => editor.set_width(Width::Word32),
        Key::Char('a') => editor.set_width(Width::ADDRESS),
        Key::Char('q') => editor.finished = true,
        _ => {}
    }
}

fn handle_mouse<W: Write>(me: MouseEvent, editor: &mut Editor<W>) {
    eprintln!("{:?}", me);
    match me {
        MouseEvent::Press(MouseButton::WheelUp, _, _) => editor.scroll(-1),
        MouseEvent::Press(MouseButton::WheelDown, _, _) => editor.scroll(1),
        _ => {}
    }
}

fn main() -> Result<(), io::Error> {
    let mut data_store = if let Some(arg) = env::args().nth(1) {
        if let Ok(n_bytes) = arg.parse() {
            DataStore::anon(n_bytes)?
        } else {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(arg)?;

            DataStore::file(file)?
        }
    } else {
        DataStore::anon(1024)?
    };

    let stdout: MouseTerminal<_> = stdout().into_raw_mode()?.into();
    let writer = BufWriter::new(stdout);
    let (width, height) = termion::terminal_size()?;
    let mut editor = Editor::new(&mut data_store, writer, width as usize, height as usize);
    editor.init();

    let stdin = stdin();
    for evt in stdin.events() {
        match evt? {
            Event::Key(key) => handle_key(key, &mut editor),
            Event::Mouse(me) => handle_mouse(me, &mut editor),
            _ => {}
        }

        if editor.finished {
            break;
        }
        editor.draw();
    }

    Ok(())
}
