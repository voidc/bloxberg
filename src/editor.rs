use std::*;
use std::io::Write;

use std::cmp::{min, max};

use crate::cell::*;
use std::cell::RefCell;
use crate::util::cmp_range;
use std::ops::Range;
use std::mem::size_of;

const N_COLS: usize = 16;
const PADDING_TOP: usize = 1;
const PADDING_BOTTOM: usize = 1;

#[derive(Debug, Copy, Clone)]
struct Line {
    offset: usize,
    len: usize,
    cpb: usize,
    min_cpb: usize,
    buddy: Buddy,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Buddy {
    None,
    Above,
    Below,
}

impl Line {
    fn new(offset: usize, len: usize, cpb: usize, min_cpb: usize, buddy: Buddy) -> Self {
        Line {
            offset,
            len,
            cpb,
            min_cpb,
            buddy,
        }
    }

    fn cell_range(&self) -> Range<usize> {
        self.offset..self.offset+self.len
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum EditorMode {
    Normal,
    Insert,
    Command,
}

pub struct Editor<W: Write> {
    stdout: RefCell<W>,
    pub height: usize,
    mode: EditorMode,
    scroll: usize,
    cursor_x: usize,
    cursor_y: usize,
    cursor_offset: usize,
    cells: Vec<Cell>,
    lines: Vec<Line>,
    cmd_buf: String,
    pub finished: bool,
    dirty: bool,
}

impl<W: Write> Editor<W> {
    pub fn new(stdout: W, height: usize, n_bytes: usize) -> Self {
        let cells = (0..n_bytes)
            .map(|i| Cell::new_hex(i, i % N_COLS))
            .collect::<Vec<Cell>>();
        let lines = cells.chunks(N_COLS)
            .map(|c| Line::new(c[0].offset, c.len(), 1, 1, Buddy::None))
            .collect::<Vec<Line>>();

        Editor {
            stdout: RefCell::new(stdout),
            height: height - PADDING_TOP - PADDING_BOTTOM,
            mode: EditorMode::Normal,
            scroll: 0,
            cursor_x: 0,
            cursor_y: 0,
            cursor_offset: 0,
            cells,
            lines,
            cmd_buf: String::new(),
            finished: false,
            dirty: false,
        }
    }

    pub fn init(&mut self, data: &[u8]) {
        self.write(format_args!("{}{}", termion::clear::All, termion::cursor::Hide));
        self.set_cursor(0, 0);
        self.draw(&data);
    }

    pub fn set_mode(&mut self, mode: EditorMode) {
        self.mode = mode;
    }

    pub fn is_cmd(&self) -> bool {
        self.mode == EditorMode::Command
    }

    pub fn is_ins(&self) -> bool {
        self.mode == EditorMode::Insert
    }

    pub fn write(&self, args: fmt::Arguments) {
        self.stdout.borrow_mut().write_fmt(args).unwrap();
    }

    fn cells(&self, line_idx: usize) -> &[Cell] {
        &self.cells[self.lines[line_idx].cell_range()]
    }

    fn cells_mut(&mut self, line_idx: usize) -> &mut [Cell] {
        &mut self.cells[self.lines[line_idx].cell_range()]
    }

    fn cell_index_at_col(&self, line_idx: usize, col: usize) -> Option<usize> {
        let idx = self.lines[line_idx].offset + col / self.lines[line_idx].cpb;
        Some(self.cells.get(idx)?.base_offset())
    }

    fn cell_index_at_cursor(&self) -> usize {
        self.cell_index_at_col(self.cursor_y, self.cursor_x).unwrap()
    }

    fn cell_at_col(&self, line_idx: usize, col: usize) -> Option<&Cell> {
        Some(&self.cells[self.cell_index_at_col(line_idx, col)?])
    }

    fn cell_at_col_mut(&mut self, line_idx: usize, col: usize) -> Option<&mut Cell> {
        let idx = self.cell_index_at_col(line_idx, col)?;
        Some(&mut self.cells[idx])
    }

    fn cell_at_cursor(&self) -> &Cell {
        self.cell_at_col(self.cursor_y, self.cursor_x).unwrap()
    }

    fn cell_at_cursor_mut(&mut self) -> &mut Cell {
        self.cell_at_col_mut(self.cursor_y, self.cursor_x).unwrap()
    }

    pub fn move_cursor_next(&mut self) {
        let line = &self.lines[self.cursor_y];
        let cell = self.cell_at_cursor();

        let mut new_cell_idx = cell.offset + cell.n_bytes();
        let mut new_y = self.cursor_y;

        if new_cell_idx >= line.offset + line.len {
            if self.cursor_y < self.lines.len() - 1 {
                new_y += 1;
            } else {
                new_cell_idx = self.cells.len() - 1;
            }
        }

        let new_x = self.cells[new_cell_idx].col; //?
        self.set_cursor(new_x, new_y);
    }

    pub fn move_cursor_prev(&mut self) {
        let line = &self.lines[self.cursor_y];
        let cell = self.cell_at_cursor();

        if cell.offset < 1 {
            return;
        }

        let mut new_cell_idx = self.cells[cell.offset - 1].base_offset();
        let mut new_y = self.cursor_y;

        if new_cell_idx < line.offset {
            if self.cursor_y > 0 {
                new_y -= 1;
            } else {
                new_cell_idx = 0;
            }
        }

        let new_x = self.cells[new_cell_idx].col;
        self.set_cursor(new_x, new_y);
    }

    pub fn move_cursor_y(&mut self, dy: isize) {
        let mut new_y = self.cursor_y as isize + dy;

        if new_y < 0 {
            new_y = 0;
        } else if new_y >= self.lines.len() as isize {
            new_y = (self.lines.len() - 1) as isize;
        }

        self.set_cursor(self.cursor_x, new_y as usize);
    }

    pub fn set_cursor_offset(&mut self, offset: usize) -> Result<(), usize> {
        let line_idx = self.lines.binary_search_by(|line| { // FIXME
            cmp_range(offset, line.cell_range())
        })?;
        let col = self.cells[offset].col;
        //let col = (offset - self.lines[line_idx].offset) * self.lines[line_idx].cpb;
        self.set_cursor(col, line_idx);
        Ok(())
    }

    pub fn set_cursor_end(&mut self) {
        let y = self.lines.len() - 1;
        let x = self.cells.last().unwrap().col;
        self.set_cursor(x, y);
    }

    pub fn set_cursor(&mut self, x: usize, y: usize) {
        self.cursor_offset = 0;
        self.cursor_x = x;
        self.cursor_y = y;

        if y < self.scroll {
            self.scroll = y;
        } else if y >= self.scroll + self.height {
            self.scroll = y - self.height + 1;
        }
    }

    pub fn switch_format(&mut self, rev: bool) {
        self.set_format(self.cell_at_cursor().format.cycle(rev));
    }

    pub fn set_format(&mut self, format: Format) {
        let cell = self.cell_at_cursor().clone();
        if cell.format == format || cell.n_bytes() * format.cols_per_byte() > N_COLS {
            return;
        }
        self.cell_at_cursor_mut().format = format;

        let min_cell = self.cells(self.cursor_y).iter()
            .max_by_key(|c| c.format.cols_per_byte())
            .unwrap()
            .clone();
        let min_cpb = min_cell.format.cols_per_byte();

        if min_cpb < self.lines[self.cursor_y].cpb {
            self.lines[self.cursor_y].min_cpb = min_cpb;
            self.merge_lines(self.cursor_y);
        } else if min_cpb > self.lines[self.cursor_y].cpb {
            self.split_line(self.cursor_y, min_cell.offset, min_cpb);
        }
        self.set_cursor_offset(cell.offset);
    }

    fn split_line(&mut self, line_idx: usize, offset: usize, min_cpb: usize) {
        if min_cpb > self.lines[line_idx].cpb {
            let line = &mut self.lines[line_idx];
            if line.len * min_cpb <= N_COLS {
                assert_eq!(line_idx, self.lines.len() - 1); // may only occur in last line
                return;
            }

            line.cpb *= 2;
            let len = N_COLS / line.cpb;
            let mut new_line = Line::new(line.offset + len, line.len - len, line.cpb, line.min_cpb, line.buddy);
            line.len = len; // new_line.len < len for last (underfull) line
            if offset < new_line.offset {
                // recalc new_line.min_cpb
                line.buddy = Buddy::Below;
                self.lines.insert(line_idx + 1, new_line);
                self.split_line(line_idx, offset, min_cpb);
            } else {
                new_line.buddy = Buddy::Above;
                self.lines.insert(line_idx + 1, new_line);
                self.split_line(line_idx + 1, offset, min_cpb);
            }
        }
    }

    fn merge_lines(&mut self, line_idx: usize) {
        if self.lines[line_idx].min_cpb < self.lines[line_idx].cpb {
            match self.lines[line_idx].buddy {
                Buddy::Above => {
                    let line = self.lines[line_idx - 1].clone();
                    let buddy = &mut self.lines[line_idx - 1];
                    if buddy.min_cpb < buddy.cpb {
                        assert_eq!(buddy.cpb, line.cpb);
                        buddy.cpb /= 2;
                        buddy.min_cpb = max(line.min_cpb, buddy.min_cpb);
                        buddy.len += line.len;
                        assert!(buddy.len * buddy.cpb <= N_COLS); // == except last line
                        self.lines.remove(line_idx);
                        self.merge_lines(line_idx - 1);
                    }
                },
                Buddy::Below => {
                    let buddy = self.lines[line_idx + 1].clone();
                    let line = &mut self.lines[line_idx];
                    if buddy.min_cpb < buddy.cpb {
                        assert_eq!(buddy.cpb, line.cpb);
                        line.cpb /= 2;
                        line.min_cpb = max(line.min_cpb, buddy.min_cpb);
                        line.buddy = buddy.buddy;
                        line.len += buddy.len;
                        assert!(line.len * line.cpb <= N_COLS); // == except last line
                        self.lines.remove(line_idx + 1);
                        self.merge_lines(line_idx);
                    }
                },
                Buddy::None => {
                    assert_eq!(line_idx, self.lines.len() - 1); // may only occur in last line
                    let line = &mut self.lines[line_idx];
                    line.cpb = line.min_cpb;
                    assert!(line.len * line.cpb <= N_COLS);
                },
            }
        }
    }

    pub fn switch_byte_order(&mut self) {
        let cell = self.cell_at_cursor_mut();
        cell.byte_order = cell.byte_order.toggle();
    }

    pub fn inc_width(&mut self) {
        self.set_width(self.cell_at_cursor().width.inc());
    }

    pub fn dec_width(&mut self) {
        self.set_width(self.cell_at_cursor().width.dec());
    }

    pub fn set_width(&mut self, width: Width) {
        let old_cell = self.cell_at_cursor().clone();
        if old_cell.width == width || width.n_bytes() * old_cell.format.cols_per_byte() > N_COLS {
            return;
        }

        let offset = width.align(old_cell.offset);
        let n_bytes = max(width.n_bytes(), old_cell.n_bytes());

        for cell in self.cells[offset..(offset + n_bytes)].iter_mut() {
            cell.width = width;
            cell.format = old_cell.format;
            cell.byte_order = old_cell.byte_order;
        }
    }

    pub fn insert(&mut self, c: char, data: &mut [u8]) {
        let cell = self.cell_at_cursor();
        if cell.format != Format::Hex || !c.is_ascii_hexdigit() {
            return;
        }

        let old = data[cell.offset];
        let new = c.to_digit(16).unwrap() as u8;

        if self.cursor_offset == 0 {
            data[cell.offset] = (old & 0x0f) | (new << 4);
            self.cursor_offset = 1;
        } else if self.cursor_offset == 1 {
            data[cell.offset] = (old & 0xf0) | new;
            self.cursor_offset = 0;
            self.move_cursor_next();
        }

        self.dirty = true;
    }

    pub fn follow_pointer(&mut self, data: &[u8]) {
        let cell = self.cell_at_cursor();
        if cell.width.n_bytes() != size_of::<usize>() {
            return;
        }
        let offset = cell.parse_value(&data[cell.offset..]) as usize;
        self.set_cursor_offset(offset);
    }

    pub fn type_cmd(&mut self, c: char) {
        if c == '\n' {
            match &self.cmd_buf[..] {
                "w" => self.dirty = false,
                "q" => self.finished = true,
                cmd => {
                    if let Ok(offset) = usize::from_str_radix(cmd, 16) {
                        self.set_cursor_offset(offset);
                    } else {
                        eprintln!("Unknown Command: \"{}\"", cmd)
                    }
                },
            }
            self.cmd_buf.clear();
            self.mode = EditorMode::Normal;
        } else if c == '\x08' {
            self.cmd_buf.pop();
        } else {
            self.cmd_buf.push(c);
        }
    }

    fn draw_status_bar(&self) {
        self.goto(1, 1 + (PADDING_TOP + self.height) as u16);
        if self.mode == EditorMode::Command {
            self.write(format_args!(":{}", self.cmd_buf));
        } else {
            let cell = self.cell_at_cursor();
            self.write(format_args!("{:?} ({}, {}) {:#018x} {:?} {:?} {:?} {}%",
                                    self.mode,
                                    self.cursor_x,
                                    self.cursor_y,
                                    cell.offset,
                                    cell.format,
                                    cell.width,
                                    cell.byte_order,
                                    self.cursor_y * 100 / self.lines.len() as usize,
            ));
        }
    }

    fn escape_non_printable(chr: char) -> char {
        match chr {
            // line feed
            '\x0A' => '␊',
            // carriage return
            '\x0D' => '␍',
            // null
            '\x00' => '␀',
            // bell
            '\x07' => '␇',
            // backspace
            '\x08' => '␈',
            // escape
            '\x1B' => '␛',
            // tab
            '\t' => '↹',
            // space
            _ => '•',
        }
    }

    fn draw_cell(&self, cell: &Cell, selected: bool, min_cols: usize, data: &[u8]) {
        fn value_to_char(value: u128) -> Option<char> {
            let c = char::from_u32(value as u32)?;
            if c.is_ascii() && !c.is_ascii_control() {
                Some(c)
            } else {
                None
            }
        }

        assert!(data.len() >= cell.n_bytes());
        self.write(format_args!(" "));

        if selected {
            self.write(format_args!("{}", termion::color::Bg(termion::color::LightBlue)));
        }

        let cell_width = max(cell.n_cols(), min_cols) * 3 - 1;
        let value = cell.parse_value(data);
        let value_char = value_to_char(value);

        if value == 0 {
            self.write(format_args!("{}", termion::color::Fg(termion::color::LightBlack)));
        } else if value_char.is_some() {
            self.write(format_args!("{}", termion::color::Fg(termion::color::Yellow)));
        }

        match cell.format {
            Format::Hex => {
                let w = 2 * cell.n_bytes();
                self.write(format_args!("{1:2$}{:03$x}", value, "", cell_width - w, w));
            }
            Format::Dec => {
                self.write(format_args!("{:>1$}", value, cell_width));
            }
            Format::SDec => {
                self.write(format_args!("{:>1$}", cell.parse_value_signed(data), cell_width));
            }
            Format::Oct => {
                let w = 4 * cell.n_bytes();
                self.write(format_args!("{1:2$}{:03$o}", value, "", cell_width - w, w));
            }
            Format::Bin => {
                let w = 8 * cell.n_bytes();
                self.write(format_args!("{1:2$}{:03$b}", value, "", cell_width - w, w));
            }
            Format::Char => {
                self.write(format_args!("{:>1$}", value_char.unwrap_or('.'), cell_width));
            }
        }

        self.write(format_args!("{}{}",
                                termion::color::Bg(termion::color::Reset),
                                termion::color::Fg(termion::color::Reset),
        ));
    }

    fn draw_header(&self) {
        self.goto(1, 1);
        self.write(format_args!("{0:1$}", "", 18));
        let cpb = self.lines[self.cursor_y].cpb;
        for i in 0..(N_COLS / cpb) {
            if self.cursor_x / cpb == i {
                self.write(format_args!(" {}{3:4$}{:02x}{}",
                                        termion::color::Fg(termion::color::LightBlue),
                                        i,
                                        termion::color::Fg(termion::color::Reset),
                                        "", (cpb - 1) * 3,
                ));
            } else {
                self.write(format_args!(" {1:2$}{:02x}", i, "", (cpb - 1) * 3));
            }
        }
    }

    fn draw_offset(&self, line_idx: usize, offset: usize) {
        if line_idx == self.cursor_y {
            self.write(format_args!("{}{:#018x}{}",
                                    termion::color::Fg(termion::color::LightBlue),
                                    offset,
                                    termion::color::Fg(termion::color::Reset),
            ));
        } else {
            self.write(format_args!("{:#018x}", offset));
        }
    }

    fn draw_line_ascii(&self, data: &[u8]) {
        self.write(format_args!(" {}", String::from_utf8_lossy(data)));
    }

    pub fn draw(&mut self, data: &[u8]) {
        self.write(format_args!("{}", termion::clear::All));
        self.draw_header();

        let mut offset = self.lines[self.scroll].offset;

        let mut i = self.scroll;
        while i < min(self.lines.len(), self.scroll + self.height) {
            assert!(self.lines[i].cell_range().end > offset);

            self.goto(1, 1 + (PADDING_TOP + i - self.scroll) as u16);
            self.draw_offset(i, offset);

            self.lines[i].offset = offset;

            let mut col = 0;
            while col < N_COLS && offset < self.cells.len() {
                self.cells[offset].col = col; //?

                let cell = self.cells[offset];
                let n_cols = max(cell.n_cols(), self.lines[i].cpb * cell.n_bytes());
                let selected = self.cursor_y == i && col <= self.cursor_x && self.cursor_x < col + n_cols;
                col += n_cols;

                assert!(col <= N_COLS);

                self.draw_cell(&cell, selected, self.lines[i].cpb * cell.n_bytes(), &data[offset..]);
                offset += cell.n_bytes();
            }

            assert_eq!(self.lines[i].len, offset - self.lines[i].offset);
            self.draw_line_ascii(&data[self.lines[i].cell_range()]);

            i += 1;
        }

        self.draw_status_bar();
        self.flush();
    }

    fn goto(&self, x: u16, y: u16) {
        self.write(format_args!("{}", termion::cursor::Goto(x, y)));
    }

    fn flush(&self) {
        self.stdout.borrow_mut().flush().unwrap();
    }
}

impl<W: Write> Drop for Editor<W> {
    fn drop(&mut self) {
        self.write(format_args!("{}{}{}",
                                termion::clear::All,
                                termion::cursor::Goto(1, 1),
                                termion::cursor::Show));
        self.flush();
    }
}
