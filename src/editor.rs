use std::io::Write;
use std::*;

use std::cmp::{max, min};

use crate::cell::*;
use crate::data_store::DataStore;
use crate::disasm::DisasmView;
use crate::terminal::{Color, Terminal};
use crate::util::cmp_range;
use std::ops::Range;

const PADDING_TOP: usize = 1;
const PADDING_BOTTOM: usize = 1;
const PADDING_LEFT: usize = 2 + 2 * Width::ADDRESS.n_bytes();

#[derive(Debug, Copy, Clone)]
struct Line {
    offset: usize,
    len: usize,
    cpb: usize,
    min_cpb: usize,
    buddy: Buddy,
    level: usize,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Buddy {
    None,
    Above,
    Below,
}

impl Line {
    fn new(offset: usize, len: usize) -> Self {
        Line {
            offset,
            len,
            cpb: 1,
            min_cpb: 1,
            buddy: Buddy::None,
            level: 0,
        }
    }

    fn cell_range(&self) -> Range<usize> {
        self.offset..self.offset + self.len
    }

    fn col_to_offset(&self, col: usize) -> usize {
        self.offset + col / self.cpb
    }

    fn offset_to_col(&self, offset: usize) -> usize {
        (offset - self.offset) * self.cpb
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

pub struct Editor<'d, W: Write> {
    data_store: &'d mut DataStore,
    terminal: Terminal<W>,
    pub height: usize,
    n_cols: usize,
    mode: EditorMode,
    scroll: usize,
    cursor_x: usize,
    cursor_y: usize,
    cursor_offset: usize,
    cells: SparseCells,
    lines: Vec<Line>,
    cmd_buf: String,
    pub finished: bool,
    dirty: bool,
    disasm_view: DisasmView,
}

impl<'d, W: Write> Editor<'d, W> {
    pub fn new(data_store: &'d mut DataStore, writer: W, width: usize, height: usize) -> Self {
        let n_cols = match ((width / 2) - PADDING_LEFT) / 3 {
            0..=7 => panic!(""),
            8..=15 => 8,
            16..=31 => 16,
            32..=63 => 32,
            _ => 64,
        };

        let n_bytes = data_store.data().len();
        let cells = SparseCells::new(n_bytes);
        let lines = (0..n_bytes)
            .step_by(n_cols)
            .map(|c| Line::new(c, min(n_cols, n_bytes - c)))
            .collect::<Vec<Line>>();

        Editor {
            data_store,
            terminal: Terminal::new(writer),
            height: height - PADDING_TOP - PADDING_BOTTOM,
            n_cols,
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
            disasm_view: DisasmView::new(),
        }
    }

    pub fn init(&mut self) {
        self.terminal.init();
        self.set_cursor(0, 0);
        self.draw();
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

    fn cell_index_at_col(&self, line_idx: usize, col: usize) -> usize {
        let idx = min(
            self.lines[line_idx].col_to_offset(col),
            self.cells.len() - 1,
        );
        self.cells.get(idx).base_offset()
    }

    fn cell_index_at_cursor(&self) -> usize {
        self.cell_index_at_col(self.cursor_y, self.cursor_x)
    }

    fn cell_at_col(&self, line_idx: usize, col: usize) -> Cell {
        self.cells.get(self.cell_index_at_col(line_idx, col))
    }

    fn cell_at_col_mut(&mut self, line_idx: usize, col: usize) -> &mut Cell {
        self.cells.get_mut(self.cell_index_at_col(line_idx, col))
    }

    fn cell_at_cursor(&self) -> Cell {
        self.cell_at_col(self.cursor_y, self.cursor_x)
    }

    fn cell_at_cursor_mut(&mut self) -> &mut Cell {
        self.cell_at_col_mut(self.cursor_y, self.cursor_x)
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

        let new_x = self.lines[new_y].offset_to_col(new_cell_idx);
        self.set_cursor(new_x, new_y);
    }

    pub fn move_cursor_prev(&mut self) {
        let line = &self.lines[self.cursor_y];
        let cell = self.cell_at_cursor();

        if cell.offset < 1 {
            return;
        }

        let mut new_cell_idx = self.cells.get(cell.offset - 1).base_offset();
        let mut new_y = self.cursor_y;

        if new_cell_idx < line.offset {
            if self.cursor_y > 0 {
                new_y -= 1;
            } else {
                new_cell_idx = 0;
            }
        }

        let new_x = self.lines[new_y].offset_to_col(new_cell_idx);
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
        let line_idx = self
            .lines
            .binary_search_by(|line| cmp_range(offset, line.cell_range()).reverse())?;
        let col = self.lines[line_idx].offset_to_col(offset);
        self.set_cursor(col, line_idx);
        Ok(())
    }

    pub fn set_cursor_end(&mut self) {
        let y = self.lines.len() - 1;
        let x = self
            .lines
            .last()
            .unwrap()
            .offset_to_col(self.cells.len() - 1);
        self.set_cursor(x, y);
    }

    pub fn set_cursor(&mut self, x: usize, y: usize) {
        self.cursor_offset = 0;
        self.cursor_x = x;
        self.cursor_y = y;

        if x >= self.lines[y].len * self.lines[y].cpb {
            self.cursor_x = self.lines[y].len * self.lines[y].cpb - 1;
        }

        if y < self.scroll {
            self.scroll = y;
        } else if y >= self.scroll + self.height {
            self.scroll = y - self.height + 1;
        }
    }

    pub fn scroll(&mut self, dy: isize) {
        if self.scroll > 0 && self.scroll < self.lines.len() - 1 {
            self.scroll = ((self.scroll as isize) + dy) as usize;
        }
    }

    pub fn switch_format(&mut self, rev: bool) {
        self.set_format(self.cell_at_cursor().format.cycle(rev));
    }

    pub fn format_string(&mut self) {
        let mut last_offset = None;
        loop {
            let cell = self.cell_at_cursor();
            if let Some(last_offset) = last_offset {
                if cell.offset <= last_offset {
                    break;
                }
            }
            if cell.width != Width::Byte8 || self.data_store.data()[cell.offset] == 0 {
                break;
            }
            last_offset = Some(cell.offset);
            self.set_format(Format::Char);
            self.move_cursor_next();
        }
    }

    pub fn set_format(&mut self, format: Format) {
        let cell = self.cell_at_cursor();
        if cell.format == format || cell.n_bytes() * format.cols_per_byte() > self.n_cols {
            return;
        }
        self.cell_at_cursor_mut().format = format;

        let min_cell = self.max_cpb_cell(self.cursor_y);
        let min_cpb = min_cell.format.cols_per_byte();

        if min_cpb < self.lines[self.cursor_y].cpb {
            self.lines[self.cursor_y].min_cpb = min_cpb;
            self.merge_lines(self.cursor_y);
        } else if min_cpb > self.lines[self.cursor_y].cpb {
            self.split_line(self.cursor_y, min_cell.offset, min_cpb);
        } else {
            self.lines[self.cursor_y].min_cpb = min_cpb;
        }
        self.set_cursor_offset(cell.offset).unwrap();
    }

    fn max_cpb_cell(&self, line_idx: usize) -> Cell {
        let line_range = self.lines[line_idx].cell_range();
        line_range
            .map(|i| self.cells.get(i))
            .filter(|c| c.offset == c.base_offset())
            .max_by_key(|c| c.format.cols_per_byte())
            .unwrap()
    }

    fn split_line(&mut self, line_idx: usize, offset: usize, min_cpb: usize) {
        if min_cpb > self.lines[line_idx].cpb {
            let line = &mut self.lines[line_idx];
            if line.len * min_cpb <= self.n_cols {
                assert_eq!(line_idx, self.lines.len() - 1); // may only occur in last line
                return;
            }

            line.cpb *= 2;
            let len = self.n_cols / line.cpb;
            let mut new_line = Line {
                offset: line.offset + len,
                len: line.len - len,
                ..*line
            };
            line.len = len; // new_line.len < len for last (underfull) line
            if offset < new_line.offset {
                line.buddy = Buddy::Below;
                line.level += 1;
                self.lines.insert(line_idx + 1, new_line);
                self.split_line(line_idx, offset, min_cpb);
                self.lines[line_idx + 1].min_cpb =
                    self.max_cpb_cell(line_idx + 1).format.cols_per_byte();
            } else {
                new_line.buddy = Buddy::Above;
                new_line.level += 1;
                self.lines.insert(line_idx + 1, new_line);
                self.split_line(line_idx + 1, offset, min_cpb);
                self.lines[line_idx].min_cpb = self.max_cpb_cell(line_idx).format.cols_per_byte();
            }
        } else {
            self.lines[line_idx].min_cpb = min_cpb;
        }
    }

    fn merge_lines(&mut self, line_idx: usize) {
        if self.lines[line_idx].min_cpb < self.lines[line_idx].cpb {
            match self.lines[line_idx].buddy {
                Buddy::Above => {
                    let line = self.lines[line_idx].clone();
                    let buddy = &mut self.lines[line_idx - 1];
                    if buddy.min_cpb < buddy.cpb {
                        assert_eq!(buddy.cpb, line.cpb);
                        buddy.cpb /= 2;
                        buddy.min_cpb = max(line.min_cpb, buddy.min_cpb);
                        buddy.len += line.len;
                        assert!(buddy.len * buddy.cpb <= self.n_cols); // == except last line
                        self.lines.remove(line_idx);
                        self.merge_lines(line_idx - 1);
                    } else if buddy.level < line.level {
                        // only swap with higher level lines
                        let bb = buddy.buddy;
                        buddy.buddy = Buddy::Below;
                        self.lines[line_idx].buddy = bb;
                    }
                }
                Buddy::Below => {
                    let buddy = self.lines[line_idx + 1].clone();
                    let line = &mut self.lines[line_idx];
                    if buddy.min_cpb < buddy.cpb {
                        assert_eq!(buddy.cpb, line.cpb);
                        line.cpb /= 2;
                        line.min_cpb = max(line.min_cpb, buddy.min_cpb);
                        line.buddy = buddy.buddy;
                        line.level = buddy.level;
                        line.len += buddy.len;
                        assert!(line.len * line.cpb <= self.n_cols); // == except last line
                        self.lines.remove(line_idx + 1);
                        self.merge_lines(line_idx);
                    } else if buddy.level < line.level {
                        line.buddy = buddy.buddy;
                        self.lines[line_idx + 1].buddy = Buddy::Above;
                    }
                }
                Buddy::None => {
                    // assert_eq!(line_idx, self.lines.len() - 1); // may only occur in last line
                    // let line = &mut self.lines[line_idx];
                    // line.cpb = line.min_cpb;
                    // assert!(line.len * line.cpb <= self.n_cols);
                }
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
        let old_cell = self.cell_at_cursor();
        if old_cell.width == width
            || width.n_bytes() * old_cell.format.cols_per_byte() > self.n_cols
        {
            return;
        }

        let offset = width.align(old_cell.offset);
        let n_bytes = max(width.n_bytes(), old_cell.n_bytes());

        for i in offset..(offset + n_bytes) {
            let cell = self.cells.get_mut(i);
            cell.width = width;
            cell.format = old_cell.format;
            cell.byte_order = old_cell.byte_order;
        }
    }

    pub fn insert(&mut self, c: char) {
        let cell = self.cell_at_cursor();
        let digit = if let Some(d) = cell.format.parse_char(c) {
            d
        } else {
            return;
        };
        if let Format::UDec | Format::SDec = cell.format {
            return;
        } // unimplemented

        let data = self.data_store.data_mut();
        let cpb = cell.format.chars_per_byte();
        if self.cursor_offset < cpb * cell.n_bytes() {
            let byte_idx = match cell.byte_order {
                ByteOrder::BigEndian => self.cursor_offset / cpb,
                ByteOrder::LittleEndian => cell.n_bytes() - self.cursor_offset / cpb - 1,
            };
            let old = data[cell.offset + byte_idx];
            let pos = (cpb - self.cursor_offset % cpb - 1) as u8;
            data[cell.offset + byte_idx] = match cell.format {
                Format::Hex => (old & !(0x0f << pos * 4)) | (digit << pos * 4),
                Format::Oct => (old & !(0x07 << pos * 3)) | (digit << pos * 3),
                Format::Bin => (old & !(0x01 << pos * 1)) | (digit << pos * 1),
                Format::Char => digit,
                _ => unimplemented!(),
            };

            if self.cursor_offset == cpb * cell.n_bytes() - 1 {
                self.cursor_offset = 0;
                self.move_cursor_next();
            } else {
                self.cursor_offset += 1;
            }
        }

        self.dirty = true;
    }

    pub fn follow_pointer(&mut self) {
        let cell = self.cell_at_cursor();
        if cell.width != Width::ADDRESS {
            return;
        }
        let data = self.data_store.data();
        let offset = cell.parse_value(&data[cell.offset..]) as usize;
        self.set_cursor_offset(offset).unwrap();
    }

    pub fn type_cmd(&mut self, c: char) {
        if c == '\n' {
            let mut cmd = self.cmd_buf.splitn(2, ' ');
            match cmd.next().unwrap() {
                "w" => {
                    self.data_store.write().unwrap();
                    self.dirty = false
                }
                "q" => self.finished = true,
                "d" => {
                    let addr = self.cell_at_cursor().offset;
                    let count = cmd.next().unwrap().parse::<usize>().unwrap();
                    self.disasm_view
                        .disassemble(addr, count, self.data_store.data());
                }
                cmd => {
                    if let Ok(offset) = usize::from_str_radix(cmd, 16) {
                        self.set_cursor_offset(offset).unwrap();
                    } else {
                        eprintln!("Unknown Command: \"{}\"", cmd)
                    }
                }
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
        self.terminal
            .goto(1, 1 + (PADDING_TOP + self.height) as u16);
        if self.mode == EditorMode::Command {
            write!(self.terminal, ":{}", self.cmd_buf);
        } else {
            let cell = self.cell_at_cursor();
            write!(
                self.terminal,
                "{:?} ({}, {}) {:#018x} {:?} {:?} {:?} {}%",
                self.mode,
                self.cursor_x,
                self.cursor_y,
                cell.offset,
                cell.format,
                cell.width,
                cell.byte_order,
                self.cursor_y * 100 / self.lines.len() as usize,
            );
        }
        self.terminal.clear_line();
    }

    fn escape_non_printable(chr: char) -> char {
        match chr {
            '\x0a' => '␊', // line feed
            '\x0d' => '␍', // carriage return
            '\x00' => '␀', // null
            '\x07' => '␇', // bell
            '\x08' => '␈', // backspace
            '\x1b' => '␛', // escape
            '\t' => '↹',   // tab
            _ => '•',      // space
        }
    }

    fn draw_cell(&self, cell: &Cell, selected: bool, min_cols: usize) {
        let data = &self.data_store.data()[cell.offset..];
        assert!(data.len() >= cell.n_bytes());
        write!(self.terminal, " ");

        if selected {
            self.terminal.bg_color(Color::Selected);
        }

        let cell_width = max(cell.n_cols(), min_cols) * 3 - 1;
        let value = cell.format(cell.parse_value(data));

        let fg_color = if value.is_null() {
            Color::Null
        } else if value.is_ascii() {
            Color::Ascii
        } else {
            Color::Default
        };
        self.terminal.fg_color(fg_color);

        if cell.supports_cursor() && selected && self.is_ins() {
            let (pre, cur, suf) = value.split(self.cursor_offset);
            let w = cell.n_chars();

            write!(self.terminal, "{:1$}", "", cell_width - w);
            if let Some(pre) = pre {
                write!(self.terminal, "{:1$}", pre, self.cursor_offset);
            }

            self.terminal.fg_color(Color::Cursor);
            write!(self.terminal, "{:1$}", cur, 1);
            self.terminal.fg_color(fg_color);

            if let Some(suf) = suf {
                write!(self.terminal, "{:1$}", suf, w - self.cursor_offset - 1);
            }
        } else {
            write!(self.terminal, "{:1$}", value, cell_width);
        }

        self.terminal.reset_color();
    }

    fn draw_header(&self, padding: usize) {
        self.terminal.goto(1, 1);
        write!(self.terminal, "{0:1$}", "", padding);
        let cpb = self.lines[self.cursor_y].cpb;
        for i in 0..(self.n_cols / cpb) {
            if self.cursor_x / cpb == i {
                write_color!(
                    self.terminal,
                    Color::Selected,
                    " {1:2$}{:02x}",
                    i,
                    "",
                    (cpb - 1) * 3
                );
            } else {
                write!(self.terminal, " {1:2$}{:02x}", i, "", (cpb - 1) * 3);
            }
        }
        self.terminal.clear_line();
    }

    fn draw_offset(&self, line_idx: usize, offset: usize) {
        if line_idx == self.cursor_y {
            write_color!(self.terminal, Color::Selected, "{:#018x}", offset);
        } else {
            write!(self.terminal, "{:#018x}", offset);
        }
    }

    fn draw_line_ascii(&self, range: Range<usize>) {
        let data = &self.data_store.data()[range];
        write!(self.terminal, " {}", String::from_utf8_lossy(data));
    }

    pub fn draw(&mut self) {
        self.draw_header(PADDING_LEFT);

        let mut offset = self.lines[self.scroll].offset;

        let mut i = self.scroll;
        while i < min(self.lines.len(), self.scroll + self.height) {
            assert!(self.lines[i].cell_range().end > offset);

            self.terminal
                .goto(1, 1 + (PADDING_TOP + i - self.scroll) as u16);
            self.draw_offset(i, offset);

            /*
            let bi = match self.lines[i].buddy {
                Buddy::Above => "^",
                Buddy::Below => "v",
                Buddy::None => "-",
            };
            write!(self.terminal, " {}{}{}{}",
                                    self.lines[i].min_cpb,
                                    self.lines[i].cpb,
                                    self.lines[i].level,
                                    bi);
             */

            self.lines[i].offset = offset;

            let mut col = 0;
            while col < self.n_cols && offset < self.cells.len() {
                assert_eq!(self.lines[i].offset_to_col(offset), col);
                assert_eq!(self.lines[i].col_to_offset(col), offset);

                let cell = self.cells.get(offset);
                let n_cols = max(cell.n_cols(), self.lines[i].cpb * cell.n_bytes());
                let selected =
                    self.cursor_y == i && col <= self.cursor_x && self.cursor_x < col + n_cols;
                col += n_cols;

                assert!(col <= self.n_cols);

                self.draw_cell(&cell, selected, self.lines[i].cpb * cell.n_bytes());
                offset += cell.n_bytes();
            }

            if self.lines[i].len != offset - self.lines[i].offset {
                eprintln!(
                    "Line {:x}: len={} offset={}",
                    i,
                    self.lines[i].len,
                    offset - self.lines[i].offset
                )
            }
            //self.draw_line_ascii(self.lines[i].cell_range());

            if self.disasm_view.is_enabled() {
                let cursor_offset = self.cell_at_cursor().offset;
                let relative_scroll = i as isize - self.cursor_y as isize;
                if let Some(insn) = self.disasm_view.get(cursor_offset, relative_scroll) {
                    if self.cursor_y == i {
                        write_color!(self.terminal, Color::Selected, " {}", insn);
                    } else {
                        write!(self.terminal, " {}", insn);
                    }
                }
            }

            self.terminal.clear_line();

            i += 1;
        }

        self.draw_status_bar();
        self.terminal.flush();
    }
}
