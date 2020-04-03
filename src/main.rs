use std::io;
use std::io::{stdin, stdout, Write};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use crate::cell::{Format, Width};
use crate::editor::*;

mod editor;
mod cell;
mod util;

fn handle_key<W: Write>(key: Key, editor: &mut Editor<W>, data: &mut [u8]) {
    match key {
        Key::Esc => editor.set_mode(EditorMode::Normal),
        Key::Char(c) if editor.mode == EditorMode::Command => editor.type_cmd(c),
        Key::Char(c) if editor.mode == EditorMode::Insert => editor.insert(c, data),
        Key::Char(':') => editor.set_mode(EditorMode::Command),
        Key::Char('i') => editor.set_mode(EditorMode::Insert),
        Key::Right | Key::Char('l') => editor.move_cursor(1, 0),
        Key::Left | Key::Char('h') => editor.move_cursor(-1, 0),
        Key::Down | Key::Char('j') => editor.move_cursor(0, 1),
        Key::Up | Key::Char('k') => editor.move_cursor(0, -1),
        Key::PageDown => editor.move_cursor(0, editor.height as isize),
        Key::PageUp => editor.move_cursor(0, -(editor.height as isize)),
        Key::Home => editor.set_cursor(0, 0),
        Key::End => editor.set_cursor_end(),
        Key::Char('f') => editor.switch_format(false),
        Key::Char('F') => editor.switch_format(true),
        Key::Char('x') => editor.set_format(Format::Hex),
        Key::Char('d') => editor.set_format(Format::SDec),
        Key::Char('u') => editor.set_format(Format::Dec),
        Key::Char('e') => editor.switch_byte_order(),
        Key::Char('+') => editor.inc_width(),
        Key::Char('-') => editor.dec_width(),
        Key::Char('b') => editor.set_width(Width::Byte8),
        Key::Char('w') => editor.set_width(Width::Word32),
        Key::Char('q') => editor.finished = true,
        _ => {}
    }
}

fn main() -> Result<(), io::Error> {
    // kitty --hold sh -c "tty"
    // kitty sh -c "reptyr pid"
    // gdb set inferior-tty /dev/pts/tty
    eprintln!("{}", std::process::id());

    //let path = env::args().nth(1).expect("Missing filename.");
    //let mut file = fs::File::open(path)?;
    let mut data = [0x00_u8; 2560];

    let stdout = stdout().into_raw_mode()?;
    let (_width, height) = termion::terminal_size()?;
    let mut editor = Editor::new(stdout, height as usize, data.len());
    editor.init(&data);

    let stdin = stdin();
    for c in stdin.keys() {
        handle_key(c?, &mut editor, &mut data);
        if editor.finished {
            break;
        }
        editor.draw(&data);
    }

    Ok(())
}
