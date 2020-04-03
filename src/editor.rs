use std::*;
use std::io::Write;

use std::cmp::{min, max};

use crate::cell::*;
use std::cell::RefCell;
use crate::util::{cmp_range, UtilExt, either};

const N_COLS: usize = 16;
const PADDING_TOP: usize = 1;
const PADDING_BOTTOM: usize = 1;

/* Strategy 1: Unaligned, Free flow
 * - arbitrary line offsets,
 * - underfull lines (long cell at the end)
 *
 * Strategy 2: Aligned, padded cells
 * - fixed cell width per line (max cols of cells)
 * - padded cells
 * - split lines if cell size inc, merge if dec
 */



struct Line {
    offset: usize,
    min_cpb: usize,
    cells: Vec<Cell>,
}

impl Line {
    fn new(offset: usize, min_cpb: usize, cells: Vec<Cell>) -> Self {
        Line {
            offset,
            min_cpb,
            cells,
        }
    }

    fn empty(offset: usize, n_bytes: usize) -> Self {
        let mut cells = Vec::new();
        for i in 0..n_bytes {
            cells.push(Cell::new_hex(offset + i, i))
        }

        Self::new(offset, 1, cells)
    }

    fn empty_vec(n_bytes: usize) -> Vec<Self> {
        let mut offset = 0;
        let mut vec = Vec::new();
        while offset < n_bytes {
            let b = min(n_bytes - offset, N_COLS);
            vec.push(Line::empty(offset, b));
            offset += b;
        }

        vec
    }

    fn cell_at_col(&self, col: usize) -> &Cell {
        let i = self.col_to_index(col);
        &self.cells[i]
    }

    fn cell_at_col_mut(&mut self, col: usize) -> &mut Cell {
        let i = self.col_to_index(col);
        &mut self.cells[i]
    }

    fn cell_at_offset(&self, offset: usize) -> &Cell {
        let i = self.offset_to_index(offset);
        &self.cells[i]
    }

    fn cell_at_offset_mut(&mut self, offset: usize) -> &mut Cell {
        let i = self.offset_to_index(offset);
        &mut self.cells[i]
    }

    fn cell_cols(&self, cell_idx: usize) -> usize {
        let cell = &self.cells[cell_idx];
        max(self.min_cpb * cell.n_bytes(), cell.n_cols())
    }

    fn col_to_index(&self, col: usize) -> usize {
        // self.cells.binary_search_by(|cell| {
        //     let cols = max(self.min_cpb * cell.n_bytes(), cell.n_cols());
        //     cmp_range(col, cell.col..(cell.col + cols))
        // }).apply(either)
        let mut x = 0;
        let mut i = 0;
        loop {
            x += self.cell_cols(i);
            i += 1;
            if x > col || i >= self.cells.len() {
                break;
            }
        }
        return i - 1;
    }

    fn offset_to_index(&self, offset: usize) -> usize {
        let mut x = self.offset;
        let mut i = 0;
        loop {
            x += self.cells[i].n_bytes();
            i += 1;
            if x > offset || i >= self.cells.len() {
                break;
            }
        }
        return i - 1;
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
    pub mode: EditorMode,
    scroll: usize,
    cursor_x: usize,
    cursor_y: usize,
    cursor_offset: usize,
    lines: Vec<Line>,
    cmd_buf: String,
    pub finished: bool,
    dirty: bool,
}

impl<W: Write> Editor<W> {
    pub fn new(stdout: W, height: usize, n_bytes: usize) -> Self {
        Editor {
            stdout: RefCell::new(stdout),
            height: height - PADDING_TOP - PADDING_BOTTOM,
            mode: EditorMode::Normal,
            scroll: 0,
            cursor_x: 0,
            cursor_y: 0,
            cursor_offset: 0,
            lines: Line::empty_vec(n_bytes),
            cmd_buf: String::new(),
            finished: false,
            dirty: false,
        }
    }

    pub fn init(&mut self, data: &[u8]) {
        self.write(format_args!("{}{}", termion::clear::All, termion::cursor::Hide));
        self.move_cursor(0, 0);
        self.draw(&data);
    }

    pub fn set_mode(&mut self, mode: EditorMode) {
        self.mode = mode;
    }

    pub fn write(&self, args: fmt::Arguments) {
        self.stdout.borrow_mut().write_fmt(args).unwrap();
    }

    fn cell_at_cursor(&self) -> &Cell {
        self.lines[self.cursor_y].cell_at_col(self.cursor_x)
    }

    fn cell_at_cursor_mut(&mut self) -> &mut Cell {
        self.lines[self.cursor_y].cell_at_col_mut(self.cursor_x)
    }

    pub fn move_cursor(&mut self, dx: isize, dy: isize) {
        let line = &self.lines[self.cursor_y];
        let cell_idx = line.col_to_index(self.cursor_x);

        let mut new_cell_idx = cell_idx as isize + dx;
        let mut new_y = self.cursor_y as isize + dy;

        if new_cell_idx < 0 {
            if self.cursor_y > 0 {
                new_cell_idx = (self.lines[self.cursor_y - 1].cells.len() - 1) as isize;
                new_y -= 1;
            } else {
                new_cell_idx = 0;
            }
        } else if new_cell_idx >= line.cells.len() as isize {
            if self.cursor_y < self.lines.len() - 1 {
                new_cell_idx = 0;
                new_y += 1;
            } else {
                new_cell_idx = (line.cells.len() - 1) as isize;
            }
        }

        if new_y < 0 {
            new_y = 0;
        } else if new_y >= self.lines.len() as isize {
            new_y = (self.lines.len() - 1) as isize;
        }

        self.set_cursor(self.lines[new_y as usize].cells[new_cell_idx as usize].col, new_y as usize);
    }

    pub fn set_cursor(&mut self, x: usize, y: usize) {
        self.cursor_offset = 0;
        self.cell_at_cursor_mut().selected = false;
        self.cursor_x = x;
        self.cursor_y = y;
        self.cell_at_cursor_mut().selected = true;

        if y < self.scroll {
            self.scroll = y;
        } else if y >= self.scroll + self.height {
            self.scroll = y - self.height + 1;
        }
    }

    pub fn set_cursor_end(&mut self) {
        let y = self.lines.len() - 1;
        let x = self.lines[y].cells.last().unwrap().col;
        self.set_cursor(x, y);
    }

    pub fn switch_format(&mut self, rev: bool) {
        self.set_format(self.cell_at_cursor().format.cycle(rev));
    }

    pub fn set_format(&mut self, format: Format) {
        let line = &mut self.lines[self.cursor_y];
        let cell_idx = line.col_to_index(self.cursor_x);
        if line.cells[cell_idx].format == format
            || line.cells[cell_idx].n_bytes() * format.cols_per_byte() > N_COLS {
            return;
        }

        line.cells[cell_idx].format = format;
        line.min_cpb = line.cells.iter().map(|c| c.format.cols_per_byte()).max().unwrap();

        //fixed cell size
        // let cpc = line.cols_per_cell;
        // let max_cols = line.cells.iter().map(Cell::n_cols).max().unwrap();
        // if max_cols > line.cols_per_cell {
        //     let new_line = Line::new(line.offset, cpc * 2, line.cells.split_off(line.cells.len() / 2));
        //     self.lines.insert(self.cursor_y, new_line);
        //     //split l2(max_cols) - l2(cpc) times
        // } else if max_cols < line.cols_per_cell {
        //     //merge
        // }
        // line.cols_per_cell = max_cols;
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
        let line = &mut self.lines[self.cursor_y];
        let cell_idx = line.col_to_index(self.cursor_x);
        if line.cells[cell_idx].width == width
            || width.n_bytes() * line.cells[cell_idx].format.cols_per_byte() > N_COLS {
            return;
        }

        let cell = line.cells.remove(cell_idx);

        let old_w = cell.n_bytes();
        let new_w = width.n_bytes();
        if old_w < new_w {
            //merge
            let o = width.align(cell.offset);
            let i = line.offset_to_index(o);
            let col = line.cells[i].col;
            while i < line.cells.len() && line.cells[i].offset < o + new_w {
                line.cells.remove(i);
            }
            //line.cells.retain(|c| !r.contains(&c.offset));
            let new_cell = Cell::new(o, col, cell.format, width, cell.byte_order);
            line.cells.insert(i, new_cell);
        } else if old_w > new_w {
            //split
            let cols_per_cell = max(cell.format.cols_per_byte() * new_w, line.min_cpb * new_w);
            for i in 0..(old_w / new_w) {
                let o = cell.offset + i * new_w;
                let col = cell.col + i * cols_per_cell;
                let new_cell = Cell::new(o, col, cell.format, width, cell.byte_order);
                line.cells.insert(cell_idx + i, new_cell)
            }
        }
        self.cell_at_cursor_mut().selected = true;
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
            self.move_cursor(1, 0);
        }

        self.dirty = true;
    }

    pub fn type_cmd(&mut self, c: char) {
        if c == '\n' {
            match &self.cmd_buf[..] {
                "w" => self.dirty = false,
                "q" => self.finished = true,
                cmd => eprintln!("Command: \"{}\"", cmd),
            }
            self.cmd_buf.clear();
            self.mode = EditorMode::Normal;
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

    fn draw_cell(&self, cell: &Cell, min_cols: usize, data: &[u8]) {
        assert!(data.len() >= cell.n_bytes());
        self.write(format_args!(" "));

        if cell.selected {
            self.write(format_args!("{}", termion::color::Bg(termion::color::LightBlue)));
        }

        let cell_width = max(cell.n_cols(), min_cols) * 3 - 1;
        let value = cell.parse_value(data);

        if value == 0 {
            self.write(format_args!("{}", termion::color::Fg(termion::color::LightBlack)));
        } else if (value as u8).is_ascii() {
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
                let mut c = char::from_u32(value as u32).unwrap_or('.');
                if !c.is_ascii() || c.is_ascii_control() {
                    c = '.';
                }
                self.write(format_args!("{:>1$}", c, cell_width));
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
        let cpb = self.lines[self.cursor_y].min_cpb;
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

    pub fn draw(&mut self, data: &[u8]) {
        self.write(format_args!("{}", termion::clear::All));
        self.draw_header();

        let mut offset = self.lines[self.scroll].offset;

        let mut i = self.scroll;
        while i < min(self.lines.len(), self.scroll + self.height) {
            if self.lines[i].cells.is_empty() {
                self.lines.remove(i);
                continue;
            }

            self.goto(1, 1 + (PADDING_TOP + i - self.scroll) as u16);
            self.draw_offset(i, offset);

            self.lines[i].offset = offset;

            // let next_line = self.lines.iter()
            //     .skip(i)
            //     .flat_map(|l| l.cells)
            //     .take_while(|c| {
            //         x += max(c.n_cols(), self.lines[i].min_cpb * c.n_bytes());
            //         x < N_COLS
            //     })
            //     .collect::<Vec<Cell>>();

            let mut col = 0;
            let mut j = 0;
            while col < N_COLS {
                if j >= self.lines[i].cells.len() {
                    if i+1 < self.lines.len() {
                        let c = self.lines[i+1].cells.remove(0);
                        self.lines[i].cells.push(c);

                        if self.lines[i+1].cells.is_empty() {
                            self.lines.remove(i+1);
                        }
                    } else {
                        break;
                    }
                }

                self.lines[i].cells[j].offset = offset;
                self.lines[i].cells[j].col = col;

                let cell = self.lines[i].cells[j];
                col += max(cell.n_cols(), self.lines[i].min_cpb * cell.n_bytes());

                if col > N_COLS && j+1 < self.lines[i].cells.len() {
                    let mut new_line = Line::new(offset, 1, self.lines[i].cells.split_off(j+1));
                    if i == self.lines.len() - 1 {
                        new_line.min_cpb = new_line.cells.iter().map(|c| c.format.cols_per_byte()).max().unwrap();
                        self.lines.push(new_line);
                    } else {
                        new_line.cells.append(&mut self.lines[i + 1].cells);
                        new_line.min_cpb = new_line.cells.iter().map(|c| c.format.cols_per_byte()).max().unwrap();
                        self.lines[i + 1] = new_line;
                    }
                    break;
                }

                self.draw_cell(&cell, self.lines[i].min_cpb * cell.n_bytes(), &data[offset..]);
                offset += cell.n_bytes();
                j += 1;
            }

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
