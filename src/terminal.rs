use std::cell::RefCell;
use std::fmt;
use std::io::Write;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Color {
    Default,
    Selected,
    Null,
    Ascii,
    Cursor,
}

impl Color {
    fn termion(&self) -> &'static dyn termion::color::Color {
        match self {
            Color::Default => &termion::color::Reset,
            Color::Selected => &termion::color::LightBlue,
            Color::Null => &termion::color::LightBlack,
            Color::Ascii => &termion::color::Yellow,
            Color::Cursor => &termion::color::LightGreen,
        }
    }
}

pub struct Terminal<W: Write> {
    writer: RefCell<W>,
}

impl<W: Write> Terminal<W> {
    pub fn new(writer: W) -> Self {
        Terminal {
            writer: RefCell::new(writer),
        }
    }

    pub fn write_fmt(&self, args: fmt::Arguments) {
        self.writer.borrow_mut().write_fmt(args).unwrap();
    }

    pub fn flush(&self) {
        self.writer.borrow_mut().flush().unwrap();
    }

    pub fn init(&self) {
        write!(self, "{}{}", termion::clear::All, termion::cursor::Hide);
    }

    pub fn clear_line(&self) {
        write!(self, "{}", termion::clear::UntilNewline);
    }

    pub fn goto(&self, x: u16, y: u16) {
        write!(self, "{}", termion::cursor::Goto(x, y));
    }

    pub fn fg_color(&self, color: Color) {
        write!(self, "{}", termion::color::Fg(color.termion()));
    }

    pub fn bg_color(&self, color: Color) {
        write!(self, "{}", termion::color::Bg(color.termion()));
    }

    pub fn reset_color(&self) {
        write!(
            self,
            "{}{}",
            termion::color::Bg(termion::color::Reset),
            termion::color::Fg(termion::color::Reset),
        );
    }

    pub fn write_color(&self, color: Color, args: fmt::Arguments) {
        self.fg_color(color);
        self.write_fmt(args);
        write!(self, "{}", termion::color::Fg(termion::color::Reset));
    }
}

impl<W: Write> Drop for Terminal<W> {
    fn drop(&mut self) {
        write!(
            self,
            "{}{}{}",
            termion::clear::All,
            termion::cursor::Goto(1, 1),
            termion::cursor::Show
        );
        self.flush();
    }
}

macro_rules! write_color {
    ($dst:expr, $col:expr, $($arg:tt)*) => ($dst.write_color($col, format_args!($($arg)*)))
}
