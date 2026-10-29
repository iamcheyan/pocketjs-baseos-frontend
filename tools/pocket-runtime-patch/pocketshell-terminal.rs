//! PocketShell VT100/ANSI terminal model.
//!
//! PTY bytes -> AnsiParser -> ScreenBuffer -> line runs for the PocketJS app.
//! Scope is the ANSI subset that shell, top, vim and ranger need; this is not
//! an xterm clone. Kept as a separate module so the host runtime stays readable.

pub const FLAG_BOLD: u8 = 1;
pub const FLAG_INVERSE: u8 = 2;
pub const FLAG_UNDERLINE: u8 = 4;

pub const DEFAULT_FG: u32 = 0xe5e5e5;
pub const DEFAULT_BG: u32 = 0x0f172a;

/// xterm's default 16-colour palette, as 0xRRGGBB.
const PALETTE: [u32; 16] = [
    0x1e1e1e, 0xcd3131, 0x0dbc79, 0xe5e510, 0x2472c8, 0xbc3fbc, 0x11a8cd, 0xe5e5e5,
    0x666666, 0xf14c4c, 0x23d18b, 0xf5f543, 0x3b8eea, 0xd670d6, 0x29b8db, 0xffffff,
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: u32,
    pub bg: u32,
    pub flags: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Self { ch: ' ', fg: DEFAULT_FG, bg: DEFAULT_BG, flags: 0 }
    }
}

#[derive(Clone, Copy)]
pub struct CursorState {
    pub row: usize,
    pub col: usize,
    pub visible: bool,
}

pub struct ScreenBuffer {
    pub rows: usize,
    pub cols: usize,
    pub cells: Vec<Cell>,
    pub dirty: Vec<bool>,
}

impl ScreenBuffer {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self { rows, cols, cells: vec![Cell::default(); rows * cols], dirty: vec![true; rows] }
    }

    fn index(&self, row: usize, col: usize) -> usize {
        row * self.cols + col
    }

    pub fn cell(&self, row: usize, col: usize) -> Cell {
        self.cells[self.index(row, col)]
    }

    fn set(&mut self, row: usize, col: usize, cell: Cell) {
        if row >= self.rows || col >= self.cols {
            return;
        }
        let index = self.index(row, col);
        if self.cells[index] != cell {
            self.cells[index] = cell;
            self.dirty[row] = true;
        }
    }

    fn clear_line(&mut self, row: usize) {
        if row >= self.rows {
            return;
        }
        let base = row * self.cols;
        for cell in &mut self.cells[base..base + self.cols] {
            *cell = Cell::default();
        }
        self.dirty[row] = true;
    }

    pub fn clear_all(&mut self) {
        self.cells.fill(Cell::default());
        self.dirty.fill(true);
    }

    fn scroll_up(&mut self, top: usize, bottom: usize, count: usize) {
        let count = count.max(1).min(bottom - top + 1);
        for row in top..=bottom - count {
            let from = (row + count) * self.cols;
            let to = row * self.cols;
            let slice: Vec<Cell> = self.cells[from..from + self.cols].to_vec();
            self.cells[to..to + self.cols].copy_from_slice(&slice);
        }
        for row in bottom + 1 - count..=bottom {
            let base = row * self.cols;
            for cell in &mut self.cells[base..base + self.cols] {
                *cell = Cell::default();
            }
        }
        for row in top..=bottom {
            self.dirty[row] = true;
        }
    }

    fn scroll_down(&mut self, top: usize, bottom: usize, count: usize) {
        let count = count.max(1).min(bottom - top + 1);
        for row in (top + count..=bottom).rev() {
            let from = (row - count) * self.cols;
            let to = row * self.cols;
            let slice: Vec<Cell> = self.cells[from..from + self.cols].to_vec();
            self.cells[to..to + self.cols].copy_from_slice(&slice);
        }
        for row in top..top + count {
            let base = row * self.cols;
            for cell in &mut self.cells[base..base + self.cols] {
                *cell = Cell::default();
            }
        }
        for row in top..=bottom {
            self.dirty[row] = true;
        }
    }
}

#[derive(Default)]
struct Attributes {
    fg: u32,
    bg: u32,
    flags: u8,
}

impl Attributes {
    fn reset(&mut self) {
        self.fg = DEFAULT_FG;
        self.bg = DEFAULT_BG;
        self.flags = 0;
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ParserState {
    Ground,
    Escape,
    Csi,
    Osc,
}

pub struct Terminal {
    pub buffer: ScreenBuffer,
    pub cursor: CursorState,
    saved_cursor: CursorState,
    attributes: Attributes,
    scroll_top: usize,
    scroll_bottom: usize,
    wrap_pending: bool,
    auto_wrap: bool,
    state: ParserState,
    params: Vec<u32>,
    current: u32,
    has_current: bool,
    private: bool,
    /// Bytes of an incomplete UTF-8 sequence carried between reads.
    pending_utf8: Vec<u8>,
    /// Alternate screen state for ESC[?1049h/l (vim, less).
    alternate: bool,
    saved_primary: Option<ScreenBuffer>,
    pub parser_errors: u32,
    pub resize_events: u32,
    alternate_screen_active: bool,
}

impl Terminal {
    pub fn new(rows: usize, cols: usize) -> Self {
        let mut attributes = Attributes::default();
        attributes.reset();
        Self {
            buffer: ScreenBuffer::new(rows, cols),
            cursor: CursorState { row: 0, col: 0, visible: true },
            saved_cursor: CursorState { row: 0, col: 0, visible: true },
            attributes,
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            wrap_pending: false,
            auto_wrap: true,
            state: ParserState::Ground,
            params: Vec::new(),
            current: 0,
            has_current: false,
            private: false,
            pending_utf8: Vec::new(),
            alternate: false,
            saved_primary: None,
            parser_errors: 0,
            resize_events: 0,
            alternate_screen_active: false,
        }
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        if rows == self.buffer.rows && cols == self.buffer.cols {
            return;
        }
        let mut next = ScreenBuffer::new(rows, cols);
        for row in 0..rows.min(self.buffer.rows) {
            for col in 0..cols.min(self.buffer.cols) {
                next.cells[row * cols + col] = self.buffer.cells[row * self.buffer.cols + col];
            }
        }
        for row in 0..rows.min(self.buffer.rows) {
            next.dirty[row] = true;
        }
        self.buffer = next;
        self.cursor.row = self.cursor.row.min(rows.saturating_sub(1));
        self.cursor.col = self.cursor.col.min(cols.saturating_sub(1));
        self.scroll_top = 0;
        self.scroll_bottom = rows.saturating_sub(1);
        self.resize_events += 1;
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        let mut index = 0;
        while index < bytes.len() {
            let byte = bytes[index];
            match self.state {
                ParserState::Ground => {
                    if byte < 0x20 || byte == 0x7f {
                        self.control(byte);
                        index += 1;
                    } else if byte < 0x80 {
                        self.print(byte as char);
                        index += 1;
                    } else {
                        // Continue an in-flight UTF-8 sequence first; a chunk can
                        // split mid-character.
                        if !self.pending_utf8.is_empty() {
                            self.pending_utf8.push(byte);
                            index += 1;
                            let need = utf8_len(self.pending_utf8[0]);
                            if need != 0 && self.pending_utf8.len() == need {
                                let pending = std::mem::take(&mut self.pending_utf8);
                                match std::str::from_utf8(&pending) {
                                    Ok(text) => {
                                        let chars: Vec<char> = text.chars().collect();
                                        for ch in chars {
                                            self.print(ch);
                                        }
                                    }
                                    Err(_) => self.parser_errors += 1,
                                }
                            } else if need == 0 || self.pending_utf8.len() >= 4 {
                                self.pending_utf8.clear();
                                self.parser_errors += 1;
                            }
                            continue;
                        }
                        let need = utf8_len(byte);
                        if need == 0 {
                            self.parser_errors += 1;
                            index += 1;
                            continue;
                        }
                        self.pending_utf8.push(byte);
                        index += 1;
                        while self.pending_utf8.len() < need && index < bytes.len() {
                            self.pending_utf8.push(bytes[index]);
                            index += 1;
                        }
                        if self.pending_utf8.len() == need {
                            let pending = std::mem::take(&mut self.pending_utf8);
                            match std::str::from_utf8(&pending) {
                                Ok(text) => {
                                    let chars: Vec<char> = text.chars().collect();
                                    for ch in chars {
                                        self.print(ch);
                                    }
                                }
                                Err(_) => self.parser_errors += 1,
                            }
                        }
                    }
                }
                ParserState::Escape => {
                    index += 1;
                    self.state = ParserState::Ground;
                    match byte {
                        b'[' => {
                            self.state = ParserState::Csi;
                            self.params.clear();
                            self.current = 0;
                            self.has_current = false;
                            self.private = false;
                        }
                        b']' => self.state = ParserState::Osc,
                        b'(' | b')' | b'*' | b'+' => {
                            // Character-set designation: skip its single argument.
                            index += 1;
                        }
                        b'7' => self.saved_cursor = self.cursor,
                        b'8' => self.cursor = self.saved_cursor,
                        b'D' => self.line_feed(),
                        b'E' => {
                            self.cursor.col = 0;
                            self.line_feed();
                        }
                        b'M' => self.reverse_index(),
                        b'H' => self.set_tab_stop(),
                        b'=' | b'>' => {}
                        _ => {}
                    }
                }
                ParserState::Csi => {
                    index += 1;
                    match byte {
                        0x30..=0x39 => {
                            self.current = self.current.saturating_mul(10).saturating_add((byte - b'0') as u32);
                            self.has_current = true;
                        }
                        b';' | b':' => {
                            self.params.push(if self.has_current { self.current } else { 0 });
                            self.current = 0;
                            self.has_current = false;
                        }
                        b'?' => self.private = true,
                        b' ' | b'!' | b'"' | b'$' | b'\'' => {}
                        0x40..=0x7e => {
                            self.params.push(if self.has_current { self.current } else { 0 });
                            self.csi(byte as char);
                            self.state = ParserState::Ground;
                        }
                        _ => {
                            self.parser_errors += 1;
                            self.state = ParserState::Ground;
                        }
                    }
                }
                ParserState::Osc => {
                    index += 1;
                    if byte == 0x07 || byte == 0x1b {
                        self.state = ParserState::Ground;
                    }
                }
            }
        }
    }

    fn control(&mut self, byte: u8) {
        match byte {
            0x00 => {}
            0x07 => {}
            0x08 => {
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                }
                self.wrap_pending = false;
            }
            0x09 => {
                let next = ((self.cursor.col / 8) + 1) * 8;
                self.cursor.col = next.min(self.buffer.cols.saturating_sub(1));
                self.wrap_pending = false;
            }
            0x0a | 0x0b | 0x0c => self.line_feed(),
            0x0d => {
                self.cursor.col = 0;
                self.wrap_pending = false;
            }
            0x1b => self.state = ParserState::Escape,
            _ => {}
        }
    }

    fn line_feed(&mut self) {
        self.wrap_pending = false;
        if self.cursor.row == self.scroll_bottom {
            self.buffer.scroll_up(self.scroll_top, self.scroll_bottom, 1);
        } else if self.cursor.row + 1 < self.buffer.rows {
            self.cursor.row += 1;
        }
    }

    fn reverse_index(&mut self) {
        if self.cursor.row == self.scroll_top {
            self.buffer.scroll_down(self.scroll_top, self.scroll_bottom, 1);
        } else if self.cursor.row > 0 {
            self.cursor.row -= 1;
        }
    }

    fn print(&mut self, ch: char) {
        let width = char_width(ch);
        if width == 0 {
            return;
        }
        if self.wrap_pending {
            self.cursor.col = 0;
            self.line_feed();
            self.wrap_pending = false;
        }
        if self.cursor.col + width > self.buffer.cols {
            if self.auto_wrap {
                self.cursor.col = 0;
                self.line_feed();
            } else {
                self.cursor.col = self.buffer.cols.saturating_sub(width);
            }
        }
        let cell = Cell {
            ch,
            fg: self.attributes.fg,
            bg: self.attributes.bg,
            flags: self.attributes.flags,
        };
        self.buffer.set(self.cursor.row, self.cursor.col, cell);
        if width == 2 {
            self.buffer.set(self.cursor.row, self.cursor.col + 1, Cell { ch: ' ', ..cell });
        }
        if self.cursor.col + width >= self.buffer.cols {
            self.cursor.col = self.buffer.cols.saturating_sub(width);
            self.wrap_pending = true;
        } else {
            self.cursor.col += width;
        }
    }

    fn param(&self, index: usize, default: usize) -> usize {
        match self.params.get(index) {
            Some(value) if *value > 0 => *value as usize,
            _ => default,
        }
    }

    fn csi(&mut self, final_byte: char) {
        match final_byte {
            'A' => {
                let count = self.param(0, 1);
                self.cursor.row = self.cursor.row.saturating_sub(count).max(self.scroll_top);
                self.wrap_pending = false;
            }
            'B' => {
                let count = self.param(0, 1);
                self.cursor.row = (self.cursor.row + count).min(self.scroll_bottom);
                self.wrap_pending = false;
            }
            'C' => {
                let count = self.param(0, 1);
                self.cursor.col = (self.cursor.col + count).min(self.buffer.cols.saturating_sub(1));
                self.wrap_pending = false;
            }
            'D' => {
                let count = self.param(0, 1);
                self.cursor.col = self.cursor.col.saturating_sub(count);
                self.wrap_pending = false;
            }
            'E' => {
                let count = self.param(0, 1);
                self.cursor.row = (self.cursor.row + count).min(self.scroll_bottom);
                self.cursor.col = 0;
            }
            'F' => {
                let count = self.param(0, 1);
                self.cursor.row = self.cursor.row.saturating_sub(count).max(self.scroll_top);
                self.cursor.col = 0;
            }
            'G' | '`' => {
                let column = self.param(0, 1).saturating_sub(1);
                self.cursor.col = column.min(self.buffer.cols.saturating_sub(1));
                self.wrap_pending = false;
            }
            'd' => {
                let row = self.param(0, 1).saturating_sub(1);
                self.cursor.row = row.min(self.buffer.rows.saturating_sub(1));
                self.wrap_pending = false;
            }
            'H' | 'f' => {
                let row = self.param(0, 1).saturating_sub(1);
                let col = self.param(1, 1).saturating_sub(1);
                self.cursor.row = row.min(self.buffer.rows.saturating_sub(1));
                self.cursor.col = col.min(self.buffer.cols.saturating_sub(1));
                self.wrap_pending = false;
            }
            'J' => {
                let mode = self.params.first().copied().unwrap_or(0);
                match mode {
                    1 => {
                        for row in 0..self.cursor.row {
                            self.buffer.clear_line(row);
                        }
                        for col in 0..=self.cursor.col.min(self.buffer.cols.saturating_sub(1)) {
                            self.buffer.set(self.cursor.row, col, Cell::default());
                        }
                    }
                    2 | 3 => self.buffer.clear_all(),
                    _ => {
                        for col in self.cursor.col..self.buffer.cols {
                            self.buffer.set(self.cursor.row, col, Cell::default());
                        }
                        for row in self.cursor.row + 1..self.buffer.rows {
                            self.buffer.clear_line(row);
                        }
                    }
                }
                self.wrap_pending = false;
            }
            'K' => {
                let mode = self.params.first().copied().unwrap_or(0);
                match mode {
                    1 => {
                        for col in 0..=self.cursor.col.min(self.buffer.cols.saturating_sub(1)) {
                            self.buffer.set(self.cursor.row, col, Cell::default());
                        }
                    }
                    2 => self.buffer.clear_line(self.cursor.row),
                    _ => {
                        for col in self.cursor.col..self.buffer.cols {
                            self.buffer.set(self.cursor.row, col, Cell::default());
                        }
                    }
                }
            }
            'L' => {
                let count = self.param(0, 1);
                if self.cursor.row >= self.scroll_top && self.cursor.row <= self.scroll_bottom {
                    self.buffer.scroll_down(self.cursor.row, self.scroll_bottom, count);
                }
            }
            'M' => {
                let count = self.param(0, 1);
                if self.cursor.row >= self.scroll_top && self.cursor.row <= self.scroll_bottom {
                    self.buffer.scroll_up(self.cursor.row, self.scroll_bottom, count);
                }
            }
            'P' => {
                let count = self.param(0, 1);
                let row = self.cursor.row;
                for col in self.cursor.col..self.buffer.cols {
                    let source = col + count;
                    let cell = if source < self.buffer.cols { self.buffer.cell(row, source) } else { Cell::default() };
                    self.buffer.set(row, col, cell);
                }
            }
            'S' => {
                let count = self.param(0, 1);
                self.buffer.scroll_up(self.scroll_top, self.scroll_bottom, count);
            }
            'T' => {
                let count = self.param(0, 1);
                self.buffer.scroll_down(self.scroll_top, self.scroll_bottom, count);
            }
            'X' => {
                let count = self.param(0, 1);
                for col in self.cursor.col..(self.cursor.col + count).min(self.buffer.cols) {
                    self.buffer.set(self.cursor.row, col, Cell::default());
                }
            }
            'm' => self.sgr(),
            'r' => {
                let top = self.param(0, 1).saturating_sub(1);
                let bottom = self.param(1, self.buffer.rows).saturating_sub(1);
                if top < bottom && bottom < self.buffer.rows {
                    self.scroll_top = top;
                    self.scroll_bottom = bottom;
                    self.cursor.row = top;
                    self.cursor.col = 0;
                }
            }
            'h' | 'l' => {
                let enable = final_byte == 'h';
                for param in self.params.clone() {
                    if self.private {
                        match param {
                            25 => self.cursor.visible = enable,
                            7 => self.auto_wrap = enable,
                            1049 | 47 | 1047 => self.set_alternate(enable),
                            _ => {}
                        }
                    }
                }
            }
            's' => self.saved_cursor = self.cursor,
            'u' => self.cursor = self.saved_cursor,
            _ => {}
        }
    }

    fn set_alternate(&mut self, enable: bool) {
        if enable == self.alternate {
            return;
        }
        if enable {
            self.saved_primary = Some(ScreenBuffer {
                rows: self.buffer.rows,
                cols: self.buffer.cols,
                cells: self.buffer.cells.clone(),
                dirty: vec![true; self.buffer.rows],
            });
            self.buffer.clear_all();
            self.cursor.row = 0;
            self.cursor.col = 0;
            self.alternate = true;
        } else if let Some(primary) = self.saved_primary.take() {
            self.buffer = ScreenBuffer {
                rows: primary.rows,
                cols: primary.cols,
                cells: primary.cells,
                dirty: vec![true; primary.rows],
            };
            self.alternate = false;
        }
        self.alternate_screen_active = self.alternate;
    }

    fn sgr(&mut self) {
        if self.params.is_empty() {
            self.attributes.reset();
            return;
        }
        let mut index = 0;
        while index < self.params.len() {
            match self.params[index] {
                0 => self.attributes.reset(),
                1 => self.attributes.flags |= FLAG_BOLD,
                4 => self.attributes.flags |= FLAG_UNDERLINE,
                7 => self.attributes.flags |= FLAG_INVERSE,
                22 => self.attributes.flags &= !FLAG_BOLD,
                24 => self.attributes.flags &= !FLAG_UNDERLINE,
                27 => self.attributes.flags &= !FLAG_INVERSE,
                30..=37 => self.attributes.fg = PALETTE[(self.params[index] - 30) as usize],
                39 => self.attributes.fg = DEFAULT_FG,
                40..=47 => self.attributes.bg = PALETTE[(self.params[index] - 40) as usize],
                49 => self.attributes.bg = DEFAULT_BG,
                90..=97 => self.attributes.fg = PALETTE[(self.params[index] - 90 + 8) as usize],
                100..=107 => self.attributes.bg = PALETTE[(self.params[index] - 100 + 8) as usize],
                38 | 48 => {
                    let target_fg = self.params[index] == 38;
                    if let Some(kind) = self.params.get(index + 1) {
                        if *kind == 5 {
                            if let Some(value) = self.params.get(index + 2) {
                                let color = color_256(*value as u8);
                                if target_fg {
                                    self.attributes.fg = color;
                                } else {
                                    self.attributes.bg = color;
                                }
                                index += 2;
                            }
                        } else if *kind == 2 {
                            if let (Some(r), Some(g), Some(b)) =
                                (self.params.get(index + 2), self.params.get(index + 3), self.params.get(index + 4))
                            {
                                let color = ((*r as u32 & 0xff) << 16) | ((*g as u32 & 0xff) << 8) | (*b as u32 & 0xff);
                                if target_fg {
                                    self.attributes.fg = color;
                                } else {
                                    self.attributes.bg = color;
                                }
                                index += 4;
                            }
                        }
                    }
                }
                _ => {}
            }
            index += 1;
        }
    }

    fn set_tab_stop(&mut self) {}

    /// Emit changed rows as the host/app wire format.
    ///
    /// `L<TAB>row<TAB>run...` where a run is `fg:bg:flags:text`.
    pub fn take_dirty_rows(&mut self, budget: usize) -> String {
        let mut output = String::new();
        let mut emitted = 0;
        for row in 0..self.buffer.rows {
            if emitted >= budget {
                break;
            }
            if !self.buffer.dirty[row] {
                continue;
            }
            self.buffer.dirty[row] = false;
            emitted += 1;
            output.push_str("L\t");
            output.push_str(&row.to_string());
            output.push('\t');
            let mut col = 0;
            let mut first = true;
            while col < self.buffer.cols {
                let cell = self.buffer.cell(row, col);
                if cell.ch == ' ' && cell.bg == DEFAULT_BG && cell.flags == 0 {
                    col += 1;
                    continue;
                }
                let start_col = col;
                let start_fg = cell.fg;
                let start_bg = cell.bg;
                let start_flags = cell.flags;
                let mut text = String::new();
                while col < self.buffer.cols {
                    let next = self.buffer.cell(row, col);
                    if next.fg != start_fg || next.bg != start_bg || next.flags != start_flags {
                        break;
                    }
                    text.push(next.ch);
                    col += 1;
                }
                if !first {
                    output.push('\t');
                }
                first = false;
                // The start column is explicit: leading and interior blanks are
                // skipped runs, so the renderer cannot derive position by summing.
                output.push_str(&format!("{start_fg:06x}:{start_bg:06x}:{start_flags}:{start_col}:"));
                output.push_str(&escape_run(&text));
            }
            output.push('\n');
        }
        output
    }

    pub fn cursor_line(&self) -> String {
        format!(
            "C\t{}\t{}\t{}\n",
            self.cursor.row,
            self.cursor.col,
            if self.cursor.visible { 1 } else { 0 }
        )
    }

    pub fn visible_cells(&self) -> usize {
        self.buffer.rows * self.buffer.cols
    }
}

fn utf8_len(byte: u8) -> usize {
    match byte {
        0x00..=0x7f => 1,
        0xc2..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf4 => 4,
        _ => 0,
    }
}

fn char_width(ch: char) -> usize {
    let code = ch as u32;
    if code < 0x20 {
        return 0;
    }
    if (0x1100..=0x115f).contains(&code)
        || (0x2e80..=0xa4cf).contains(&code)
        || (0xac00..=0xd7a3).contains(&code)
        || (0xf900..=0xfaff).contains(&code)
        || (0xfe30..=0xfe6f).contains(&code)
        || (0xff00..=0xff60).contains(&code)
        || (0xffe0..=0xffe6).contains(&code)
        || (0x1f300..=0x1f64f).contains(&code)
        || (0x1f900..=0x1f9ff).contains(&code)
        || (0x20000..=0x3fffd).contains(&code)
    {
        2
    } else {
        1
    }
}

fn color_256(value: u8) -> u32 {
    match value {
        0..=15 => PALETTE[value as usize],
        16..=231 => {
            let index = value as u32 - 16;
            let r = index / 36;
            let g = (index % 36) / 6;
            let b = index % 6;
            let scale = |v: u32| if v == 0 { 0 } else { 55 + v * 40 };
            (scale(r) << 16) | (scale(g) << 8) | scale(b)
        }
        _ => {
            let level = 8 + (value as u32 - 232) * 10;
            (level << 16) | (level << 8) | level
        }
    }
}

fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Feed arbitrary bytes through a throwaway terminal and describe the result.
/// Used by the developer host's ANSI playground; the device never calls it.
pub fn playground(bytes: &[u8], rows: usize, cols: usize) -> (String, String) {
    let mut terminal = Terminal::new(rows, cols);
    terminal.feed(bytes);
    let mut runs = String::from("[");
    let mut first = true;
    for row in 0..rows {
        for col in 0..cols {
            let cell = terminal.buffer.cell(row, col);
            if cell.ch == ' ' && cell.bg == DEFAULT_BG && cell.flags == 0 {
                continue;
            }
            if !first {
                runs.push(',');
            }
            first = false;
            runs.push_str(&format!(
                "{{\"row\":{row},\"col\":{col},\"ch\":\"{}\",\"fg\":\"{:06x}\",\"bg\":\"{:06x}\",\"flags\":{}}}",
                json_escape(&cell.ch.to_string()),
                cell.fg,
                cell.bg,
                cell.flags
            ));
        }
    }
    runs.push(']');
    let meta = format!(
        "{{\"cursor\":[{},{}],\"visible\":{},\"parserErrors\":{}}}",
        terminal.cursor.row, terminal.cursor.col, terminal.cursor.visible as u8, terminal.parser_errors
    );
    (runs, meta)
}

fn escape_run(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }
    out
}

use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

pub const WORKSPACES: usize = 5;
pub const DEFAULT_COLS: usize = 54;
pub const DEFAULT_ROWS: usize = 24;

struct PtySession {
    master: i32,
    child: Child,
    terminal: Terminal,
    exited_reported: bool,
    pty_path: String,
    started: Instant,
    last_activity: Instant,
    bytes_in: u64,
    bytes_out: u64,
}

pub struct PocketShell {
    sessions: Vec<Option<PtySession>>,
    log: Vec<String>,
    visible: usize,
    frames: u64,
    started_at: Instant,
    last_fps_at: Instant,
    fps: f32,
    frame_ms: f32,
    rendered_cells: usize,
    rendered_runs: usize,
    rows: usize,
    cols: usize,
}

impl PocketShell {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            sessions: (0..WORKSPACES).map(|_| None).collect(),
            log: Vec::new(),
            visible: 0,
            frames: 0,
            started_at: now,
            last_fps_at: now,
            fps: 0.0,
            frame_ms: 0.0,
            rendered_cells: 0,
            rendered_runs: 0,
            rows: DEFAULT_ROWS,
            cols: DEFAULT_COLS,
        }
    }

    fn record(&mut self, message: String) {
        self.log.push(message);
        if self.log.len() > 200 {
            self.log.remove(0);
        }
    }

    /// Mark every row dirty so the next frame carries the whole screen. A newly
    /// attached browser has no row cache, so the host asks for this on connect.
    pub fn repaint(&mut self, id: usize) {
        if let Some(session) = self.sessions.get_mut(id).and_then(|slot| slot.as_mut()) {
            session.terminal.buffer.dirty.fill(true);
        }
    }

    /// Resize every workspace's terminal model. The PTY kernel size is only
    /// applied at spawn time; a live resize updates the model and is logged.
    pub fn resize(&mut self, rows: usize, cols: usize) {
        if rows == 0 || cols == 0 || (rows == self.rows && cols == self.cols) {
            return;
        }
        self.rows = rows;
        self.cols = cols;
        for session in self.sessions.iter_mut().flatten() {
            session.terminal.resize(rows, cols);
        }
        self.record(format!("Resize Events {cols}x{rows}"));
    }

    pub fn set_visible(&mut self, id: usize) {
        if id >= WORKSPACES || id == self.visible {
            return;
        }
        self.visible = id;
        if let Some(session) = self.sessions[id].as_mut() {
            session.terminal.buffer.dirty.fill(true);
        }
        self.record(format!("Workspace Activated WS{}", id + 1));
    }

    fn start(&mut self, id: usize) {
        if let Some(mut session) = self.sessions.get_mut(id).and_then(Option::take) {
            if session.child.try_wait().ok().flatten().is_none() {
                self.sessions[id] = Some(session);
                return;
            }
            unsafe { libc::close(session.master) };
            self.record(format!("PTY Exited WS{}", id + 1));
        }
        let program = match id {
            0 => "/bin/sh",
            1 => {
                if std::path::Path::new("/usr/bin/ranger").exists() {
                    "/usr/bin/ranger"
                } else if std::path::Path::new("/usr/bin/lf").exists() {
                    "/usr/bin/lf"
                } else {
                    "/bin/sh"
                }
            }
            2 => {
                if std::path::Path::new("/usr/bin/vim").exists() {
                    "/usr/bin/vim"
                } else {
                    "/bin/vi"
                }
            }
            3 => {
                if std::path::Path::new("/usr/bin/htop").exists() {
                    "/usr/bin/htop"
                } else {
                    "/usr/bin/top"
                }
            }
            _ => "/bin/sh",
        };
        unsafe {
            let mut master: libc::c_int = -1;
            let mut slave: libc::c_int = -1;
            let size = libc::winsize {
                ws_row: self.rows as u16,
                ws_col: self.cols as u16,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            if libc::openpty(&mut master, &mut slave, std::ptr::null_mut(), std::ptr::null(), &size) != 0 {
                self.record(format!("PTY Start Failed WS{}", id + 1));
                return;
            }
            let stdin_fd = libc::dup(slave);
            let stdout_fd = libc::dup(slave);
            let stderr_fd = libc::dup(slave);
            if stdin_fd < 0 || stdout_fd < 0 || stderr_fd < 0 {
                for fd in [stdin_fd, stdout_fd, stderr_fd] {
                    if fd >= 0 {
                        libc::close(fd);
                    }
                }
                libc::close(master);
                libc::close(slave);
                self.record(format!("PTY Start Failed WS{}", id + 1));
                return;
            }
            let mut command = Command::new(program);
            command
                .env("TERM", "xterm-256color")
                .env("LANG", "C.UTF-8")
                .env("COLUMNS", self.cols.to_string())
                .env("LINES", self.rows.to_string());
            command.stdin(Stdio::from(std::fs::File::from_raw_fd(stdin_fd)));
            command
                .stdout(Stdio::from(std::fs::File::from_raw_fd(stdout_fd)))
                .stderr(Stdio::from(std::fs::File::from_raw_fd(stderr_fd)));
            command.pre_exec(move || {
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::ioctl(0, libc::TIOCSCTTY, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
            match command.spawn() {
                Ok(child) => {
                    libc::close(slave);
                    let flags = libc::fcntl(master, libc::F_GETFL);
                    libc::fcntl(master, libc::F_SETFL, flags | libc::O_NONBLOCK);
                    let pty_path = {
                        let raw = libc::ptsname(master);
                        if raw.is_null() {
                            String::from("unknown")
                        } else {
                            std::ffi::CStr::from_ptr(raw).to_string_lossy().into_owned()
                        }
                    };
                    let now = Instant::now();
                    self.sessions[id] = Some(PtySession {
                        master,
                        child,
                        terminal: Terminal::new(self.rows, self.cols),
                        exited_reported: false,
                        pty_path,
                        started: now,
                        last_activity: now,
                        bytes_in: 0,
                        bytes_out: 0,
                    });
                    self.record(format!("Workspace Created WS{}; PTY Started", id + 1));
                }
                Err(error) => {
                    libc::close(master);
                    libc::close(slave);
                    self.record(format!("PTY Start Failed WS{}: {error}", id + 1));
                }
            }
        }
    }

    fn pump(&mut self) {
        let mut exited = Vec::new();
        for (id, slot) in self.sessions.iter_mut().enumerate() {
            let Some(session) = slot.as_mut() else { continue };
            let mut bytes = [0u8; 8192];
            loop {
                let count = unsafe {
                    libc::read(session.master, bytes.as_mut_ptr().cast(), bytes.len())
                };
                if count <= 0 {
                    break;
                }
                session.terminal.feed(&bytes[..count as usize]);
                session.bytes_out += count as u64;
                session.last_activity = Instant::now();
            }
            if !session.exited_reported && session.child.try_wait().ok().flatten().is_some() {
                session.exited_reported = true;
                exited.push(id);
            }
        }
        for id in exited {
            self.record(format!("PTY Exited WS{}", id + 1));
        }
    }

    pub fn command(&mut self, command: &str) -> String {
        let mut parts = command.splitn(2, '\t');
        let head = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("");
        if head == "LOG" {
            self.record(rest.to_owned());
            return String::new();
        }
        if head == "REPAINT" {
            if let Ok(id) = rest.trim().parse::<usize>() {
                self.repaint(id);
            }
            return String::new();
        }
        if head == "RESET" && rest == "0" {
            if let Some(mut session) = self.sessions[0].take() {
                unsafe { libc::kill(-(session.child.id() as i32), libc::SIGKILL); }
                let _ = session.child.kill();
                let _ = session.child.wait();
                unsafe { libc::close(session.master); }
            }
            self.start(0);
            return String::new();
        }
        if head == "RESIZE" {
            let mut size = rest.split('x');
            if let (Some(rows), Some(cols)) = (size.next(), size.next()) {
                if let (Ok(rows), Ok(cols)) = (rows.parse(), cols.parse()) {
                    self.resize(rows, cols);
                }
            }
            return String::new();
        }
        if head == "ACTIVATE" {
            if let Ok(id) = rest.parse::<usize>() {
                self.set_visible(id);
            }
            return String::new();
        }
        if head == "SNAP" {
            return self.snapshot(rest.parse().unwrap_or(self.visible));
        }
        let Ok(id) = head.parse::<usize>() else { return String::new() };
        if id >= WORKSPACES {
            return String::new();
        }
        if id == 4 {
            return self.log_snapshot();
        }
        self.start(id);
        let Some(session) = self.sessions[id].as_mut() else { return String::new() };
        let payload: Vec<u8> = if let Some(text) = rest.strip_prefix("TEXT ") {
            text.as_bytes().to_vec()
        } else if let Some(key) = rest.strip_prefix("KEY ") {
            match key {
                "ENTER" => b"\r".to_vec(),
                "BACKSPACE" => b"\x7f".to_vec(),
                "UP" => b"\x1b[A".to_vec(),
                "DOWN" => b"\x1b[B".to_vec(),
                "LEFT" => b"\x1b[D".to_vec(),
                "RIGHT" => b"\x1b[C".to_vec(),
                "HOME" => b"\x1b[H".to_vec(),
                "END" => b"\x1b[F".to_vec(),
                "PGUP" => b"\x1b[5~".to_vec(),
                "PGDN" => b"\x1b[6~".to_vec(),
                "TAB" => b"\t".to_vec(),
                "ESC" => b"\x1b".to_vec(),
                "CTRL_C" => b"\x03".to_vec(),
                "CTRL_D" => b"\x04".to_vec(),
                "CTRL_L" => b"\x0c".to_vec(),
                "X" => b"x".to_vec(),
                "Y" => b"y".to_vec(),
                "L" => b"q".to_vec(),
                "R" => b"e".to_vec(),
                "L1" => b"\x1b[5~".to_vec(),
                "R1" => b"\x1b[6~".to_vec(),
                "START" => b"\t".to_vec(),
                "SELECT" => b"`".to_vec(),
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        };
        if !payload.is_empty() {
            unsafe { libc::write(session.master, payload.as_ptr().cast(), payload.len()) };
            session.bytes_in += payload.len() as u64;
            session.last_activity = Instant::now();
        }
        String::new()
    }

    /// Frame callback: pump PTYs and return the visible workspace's dirty rows.
    pub fn frame(&mut self) -> String {
        let started = Instant::now();
        let output = self.frame_inner();
        // Every path is measured, including the log snapshot.
        let elapsed_ms = started.elapsed().as_secs_f32() * 1000.0;
        self.frame_ms = if self.frame_ms > 0.0 {
            self.frame_ms * 0.9 + elapsed_ms * 0.1
        } else {
            elapsed_ms
        };
        output
    }

    fn frame_inner(&mut self) -> String {
        self.pump();
        self.frames += 1;
        let elapsed = self.last_fps_at.elapsed();
        if elapsed.as_millis() >= 500 {
            self.fps = self.frames as f32 / elapsed.as_secs_f32();
            self.frames = 0;
            self.last_fps_at = Instant::now();
        }
        let id = self.visible;
        if id == 4 {
            // The log workspace owns no PTY; its frame is the metrics snapshot.
            return self.log_snapshot();
        }
        let Some(session) = self.sessions[id].as_mut() else { return String::new() };
        let rows = session.terminal.take_dirty_rows(24);
        let mut runs = 0usize;
        let mut cells = 0usize;
        for line in rows.lines().filter(|line| line.starts_with("L\t")) {
            for run in line.split('\t').skip(2) {
                runs += 1;
                // Wire shape is fg:bg:flags:col:text — skip the four numeric fields.
                let mut parts = run.splitn(5, ':');
                parts.next();
                parts.next();
                parts.next();
                parts.next();
                if let Some(text) = parts.next() {
                    cells += text.chars().count();
                }
            }
        }
        if !rows.is_empty() {
            self.rendered_runs = runs;
            self.rendered_cells = cells;
        }
        if rows.is_empty() {
            return session.terminal.cursor_line();
        }
        let mut output = String::with_capacity(rows.len() + 32);
        output.push_str(&rows);
        output.push_str(&session.terminal.cursor_line());
        output
    }

    fn snapshot(&mut self, id: usize) -> String {
        if id == 4 {
            return self.log_snapshot();
        }
        let Some(session) = self.sessions[id].as_mut() else { return String::new() };
        session.terminal.buffer.dirty.fill(true);
        let rows = session.terminal.take_dirty_rows(self.rows);
        let mut output = rows;
        output.push_str(&session.terminal.cursor_line());
        output
    }

    /// JSON snapshot for the developer host's inspectors. Kept here so the
    /// board diagnostics read the same state the device renders.
    pub fn inspector_json(&self) -> String {
        let mut out = String::with_capacity(1024);
        out.push_str(&format!(
            "{{\"cols\":{},\"rows\":{},\"visible\":{},\"fps\":{:.1},\"frameMs\":{:.3},\"cells\":{},\"runs\":{},\"logs\":{},",
            self.cols,
            self.rows,
            self.visible,
            self.fps,
            self.frame_ms,
            self.rendered_cells,
            self.rendered_runs,
            self.log.len()
        ));
        out.push_str("\"workspaces\":[");
        for id in 0..WORKSPACES {
            if id > 0 {
                out.push(',');
            }
            let name = match id {
                0 => "SHELL",
                1 => "FILES",
                2 => "VIM",
                3 => "TOP",
                _ => "LOG",
            };
            match self.sessions[id].as_ref() {
                Some(session) => {
                    // `exited_reported` is maintained by `frame()`; reading it keeps
                    // the inspector side-effect free.
                    let state = if session.exited_reported { "exited" } else { "running" };
                    out.push_str(&format!(
                        "{{\"id\":{id},\"name\":\"{name}\",\"kind\":\"pty\",\"state\":\"{state}\",\"pid\":{},\"pty\":\"{}\",\"cells\":{},\"uptime\":{:.1},\"idle\":{:.1},\"bytesIn\":{},\"bytesOut\":{},\"altScreen\":{}}}",
                        session.child.id(),
                        session.pty_path,
                        session.terminal.visible_cells(),
                        session.started.elapsed().as_secs_f32(),
                        session.last_activity.elapsed().as_secs_f32(),
                        session.bytes_in,
                        session.bytes_out,
                        session.terminal.alternate_screen_active,
                    ));
                }
                None => {
                    out.push_str(&format!(
                        "{{\"id\":{id},\"name\":\"{name}\",\"kind\":\"{}\",\"state\":\"idle\",\"pid\":0,\"pty\":\"-\",\"cells\":{},\"uptime\":0,\"idle\":0,\"bytesIn\":0,\"bytesOut\":0,\"altScreen\":false}}",
                        if id == 4 { "log" } else { "pty" },
                        self.cols * self.rows
                    ));
                }
            }
        }
        out.push_str("],\"log\":[");
        for (index, entry) in self.log.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&json_escape(entry));
            out.push('"');
        }
        out.push_str("]}");
        out
    }

    /// Event log only, for the log viewer pane.
    pub fn log_json(&self) -> String {
        let mut out = String::from("[");
        for (index, entry) in self.log.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&json_escape(entry));
            out.push('"');
        }
        out.push(']');
        out
    }

    fn log_snapshot(&mut self) -> String {
        let mut lines = vec![
            "PocketShell Runtime Log".to_string(),
            format!(
                "FPS {:.1}  frame {:.2}ms  cells {}  runs {}",
                self.fps, self.frame_ms, self.rendered_cells, self.rendered_runs
            ),
            format!(
                "buffer {}x{} ({} cells)  resize {}  workspaces {}  visible WS{}",
                self.cols,
                self.rows,
                self.sessions
                    .get(self.visible)
                    .and_then(|session| session.as_ref())
                    .map(|session| session.terminal.visible_cells())
                    .unwrap_or(self.cols * self.rows),
                self.sessions
                    .get(self.visible)
                    .and_then(|session| session.as_ref())
                    .map(|session| session.terminal.resize_events)
                    .unwrap_or(0),
                WORKSPACES,
                self.visible + 1
            ),
            format!("uptime {:.0}s", self.started_at.elapsed().as_secs()),
            String::new(),
        ];
        lines.extend(self.log.iter().cloned());
        let mut output = String::new();
        for (row, line) in lines.iter().enumerate().take(self.rows) {
            let fg = if row < 4 { 0x23d18b } else { 0xe5e5e5 };
            output.push_str(&format!(
                "L\t{}\t{:06x}:{:06x}:0:0:{}\n",
                row,
                fg,
                DEFAULT_BG,
                escape_run(line)
            ));
        }
        output.push_str("C\t0\t0\t0\n");
        output
    }
}

#[cfg(test)]
mod host_tests {
    use super::*;

    #[test]
    fn shell_workspace_persists_and_echoes() {
        let mut shell = PocketShell::new();
        shell.start(0);
        assert!(shell.sessions[0].is_some());
        shell.command("0\tTEXT echo pocketshell-ok");
        shell.command("0\tKEY ENTER");
        let mut captured = String::new();
        for _ in 0..100 {
            captured.push_str(&shell.frame());
            if captured.contains("pocketshell-ok") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(captured.contains("pocketshell-ok"), "got {captured:?}");
    }

    #[test]
    fn shell_state_survives_workspace_switch() {
        let mut shell = PocketShell::new();
        shell.start(0);
        shell.command("0\tTEXT cd /tmp");
        shell.command("0\tKEY ENTER");
        for _ in 0..20 {
            shell.frame();
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        shell.set_visible(1);
        shell.start(1);
        for _ in 0..10 {
            shell.frame();
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        shell.set_visible(0);
        shell.command("0\tTEXT pwd");
        shell.command("0\tKEY ENTER");
        let mut captured = String::new();
        for _ in 0..100 {
            captured.push_str(&shell.frame());
            if captured.contains("/tmp") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(captured.contains("/tmp"), "WS1 should still be in /tmp, got {captured:?}");
    }

    #[test]
    fn process_exit_is_logged() {
        let mut shell = PocketShell::new();
        shell.start(0);
        shell.command("0\tTEXT exit");
        shell.command("0\tKEY ENTER");
        let mut log = String::new();
        for _ in 0..100 {
            shell.frame();
            log = shell.log_snapshot();
            if log.contains("PTY Exited WS1") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(log.contains("PTY Exited WS1"), "got {log:?}");
    }

    #[test]
    fn log_workspace_reports_metrics() {
        let mut shell = PocketShell::new();
        shell.command("LOG\tWorkspace Created WS1");
        let log = shell.log_snapshot();
        assert!(log.contains("FPS"), "metrics missing: {log:?}");
        assert!(log.contains("Workspace Created WS1"), "log missing: {log:?}");
        assert!(log.contains("visible WS1"), "visible workspace missing: {log:?}");
    }

    #[test]
    fn top_renders_through_the_vt_layer() {
        if !std::path::Path::new("/usr/bin/top").exists() && !std::path::Path::new("/bin/top").exists() {
            return;
        }
        let mut shell = PocketShell::new();
        shell.set_visible(3);
        shell.start(3);
        let mut captured = String::new();
        for _ in 0..60 {
            captured.push_str(&shell.frame());
            if captured.contains("PID") || captured.contains("load") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(captured.contains("L\t"), "top should emit dirty rows, got {captured:?}");
        assert!(
            captured.contains("PID") || captured.contains("top") || captured.contains("%CPU"),
            "top header missing, got {captured:?}"
        );
    }

    #[test]
    fn vim_uses_alternate_screen() {
        if !std::path::Path::new("/usr/bin/vim").exists() && !std::path::Path::new("/bin/vi").exists() {
            return;
        }
        let mut shell = PocketShell::new();
        shell.set_visible(2);
        shell.start(2);
        for _ in 0..40 {
            shell.frame();
            if shell.sessions[2].as_ref().is_some_and(|s| s.terminal.alternate_screen_active) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let alternate = shell.sessions[2].as_ref().is_some_and(|s| s.terminal.alternate_screen_active);
        assert!(alternate, "vim should enter the alternate screen");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_row(terminal: &Terminal, row: usize) -> String {
        let mut text = String::new();
        for col in 0..terminal.buffer.cols {
            text.push(terminal.buffer.cell(row, col).ch);
        }
        text.trim_end().to_string()
    }

    #[test]
    fn plain_text_and_carriage_return() {
        let mut terminal = Terminal::new(3, 20);
        terminal.feed(b"hello\r\nworld");
        assert_eq!(text_row(&terminal, 0), "hello");
        assert_eq!(text_row(&terminal, 1), "world");
        assert_eq!(terminal.cursor.row, 1);
        assert_eq!(terminal.cursor.col, 5);
    }

    #[test]
    fn cursor_position_and_clear_screen() {
        let mut terminal = Terminal::new(4, 10);
        terminal.feed(b"\x1b[2J\x1b[2;3Habc");
        assert_eq!(text_row(&terminal, 1), "  abc");
        terminal.feed(b"\x1b[2J");
        assert_eq!(text_row(&terminal, 1), "");
    }

    #[test]
    fn backspace_moves_left() {
        let mut terminal = Terminal::new(2, 10);
        terminal.feed(b"abcd\x08\x08X");
        assert_eq!(text_row(&terminal, 0), "abXd");
    }

    #[test]
    fn colors_and_reset() {
        let mut terminal = Terminal::new(2, 10);
        terminal.feed(b"\x1b[31mR\x1b[0mN");
        assert_eq!(terminal.buffer.cell(0, 0).fg, PALETTE[1]);
        assert_eq!(terminal.buffer.cell(0, 1).fg, DEFAULT_FG);
    }

    #[test]
    fn bold_inverse_flags() {
        let mut terminal = Terminal::new(2, 10);
        terminal.feed(b"\x1b[1;7mX\x1b[0mY");
        assert_eq!(terminal.buffer.cell(0, 0).flags, FLAG_BOLD | FLAG_INVERSE);
        assert_eq!(terminal.buffer.cell(0, 1).flags, 0);
    }

    #[test]
    fn erase_line_and_display() {
        let mut terminal = Terminal::new(3, 10);
        terminal.feed(b"abcdef\x1b[1;3H\x1b[K");
        assert_eq!(text_row(&terminal, 0), "ab");
        terminal.feed(b"\x1b[2J");
        assert_eq!(text_row(&terminal, 0), "");
    }

    #[test]
    fn scroll_region_keeps_status_line() {
        let mut terminal = Terminal::new(4, 10);
        terminal.feed(b"\x1b[1;2r");
        terminal.feed(b"a\r\nb\r\nc\r\nd");
        assert_eq!(text_row(&terminal, 0), "c");
        assert_eq!(text_row(&terminal, 1), "d");
        assert_eq!(text_row(&terminal, 2), "");
    }

    #[test]
    fn alternate_screen_restores_primary() {
        let mut terminal = Terminal::new(3, 10);
        terminal.feed(b"primary");
        terminal.feed(b"\x1b[?1049h");
        terminal.feed(b"alt");
        assert_eq!(text_row(&terminal, 0), "alt");
        assert!(terminal.alternate_screen_active);
        terminal.feed(b"\x1b[?1049l");
        assert_eq!(text_row(&terminal, 0), "primary");
        assert!(!terminal.alternate_screen_active);
    }

    #[test]
    fn insert_and_delete_lines() {
        let mut terminal = Terminal::new(3, 10);
        terminal.feed(b"one\r\ntwo\r\nthree");
        terminal.feed(b"\x1b[2;1H\x1b[M");
        assert_eq!(text_row(&terminal, 0), "one");
        assert_eq!(text_row(&terminal, 1), "three");
    }

    #[test]
    fn utf8_split_across_chunks() {
        let mut terminal = Terminal::new(2, 10);
        let bytes = "终端".as_bytes();
        terminal.feed(&bytes[..2]);
        terminal.feed(&bytes[2..]);
        assert_eq!(terminal.buffer.cell(0, 0).ch, '终');
        assert_eq!(terminal.buffer.cell(0, 2).ch, '端');
    }

    #[test]
    fn dirty_rows_carry_runs() {
        let mut terminal = Terminal::new(2, 10);
        terminal.feed(b"\x1b[32mok\x1b[0m");
        let rows = terminal.take_dirty_rows(8);
        assert!(rows.starts_with("L\t0\t"), "got {rows:?}");
        assert!(rows.contains("0dbc79"), "fg should be green, got {rows:?}");
        assert!(rows.contains(":0:ok"), "run should carry its column, got {rows:?}");
        assert!(rows.contains("ok"));
        assert!(terminal.take_dirty_rows(8).is_empty());
    }

    #[test]
    fn cursor_visibility_and_save_restore() {
        let mut terminal = Terminal::new(3, 10);
        terminal.feed(b"\x1b[?25l");
        assert!(!terminal.cursor.visible);
        terminal.feed(b"\x1b[?25h\x1b[2;3H\x1b[s\x1b[1;1Hrestored\x1b[uZ");
        assert_eq!(terminal.buffer.cell(1, 2).ch, 'Z');
        assert_eq!(terminal.buffer.cell(0, 0).ch, 'r');
    }
}
