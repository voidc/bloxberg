use std::io::Write;
use std::cell::RefCell;
use std::fmt;

pub enum Color {
    Selected,
    Null,
    Ascii,
}

impl Color {
    fn termion(&self) -> &'static dyn termion::color::Color {
        match self {
            Color::Selected => &termion::color::LightBlue,
            Color::Null => &termion::color::LightBlack,
            Color::Ascii => &termion::color::Yellow,
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

    pub fn write(&self, args: fmt::Arguments) {
        self.writer.borrow_mut().write_fmt(args).unwrap();
    }

    pub fn flush(&self) {
        self.writer.borrow_mut().flush().unwrap();
    }

    pub fn init(&self) {
        self.write(format_args!("{}{}", termion::clear::All, termion::cursor::Hide));
    }

    pub fn clear_line(&self) {
        self.write(format_args!("{}", termion::clear::UntilNewline));
    }

    pub fn goto(&self, x: u16, y: u16) {
        self.write(format_args!("{}", termion::cursor::Goto(x, y)));
    }

    pub fn fg_color(&self, color: Color) {
        self.write(format_args!("{}", termion::color::Fg(color.termion())));
    }

    pub fn bg_color(&self, color: Color) {
        self.write(format_args!("{}", termion::color::Bg(color.termion())));
    }

    pub fn reset_color(&self) {
        self.write(format_args!("{}{}",
                                termion::color::Bg(termion::color::Reset),
                                termion::color::Fg(termion::color::Reset),
        ));
    }

    pub fn write_color(&self, color: Color, args: fmt::Arguments) {
        self.fg_color(color);
        self.write(args);
        self.write(format_args!("{}", termion::color::Fg(termion::color::Reset)));
    }
}

impl<W: Write> Drop for Terminal<W> {
    fn drop(&mut self) {
        self.write(format_args!("{}{}{}",
                                termion::clear::All,
                                termion::cursor::Goto(1, 1),
                                termion::cursor::Show));
        self.flush();
    }
}