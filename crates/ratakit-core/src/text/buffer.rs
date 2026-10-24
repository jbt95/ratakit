use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use super::atom::Atom;
use crate::Revisioned;

/// Index into the flattened [`TextBuffer::display`] string (byte offset).
pub type Position = usize;

#[derive(Debug, Clone)]
enum Piece {
    Text(String),
    Atom(Atom),
}

impl Piece {
    fn display_len(&self) -> usize {
        match self {
            Piece::Text(s) => s.len(),
            Piece::Atom(a) => a.display().len(),
        }
    }

    fn as_atom(&self) -> Option<&Atom> {
        match self {
            Piece::Text(_) => None,
            Piece::Atom(a) => Some(a),
        }
    }
}

#[derive(Debug, Clone)]
struct Snapshot {
    pieces: Vec<Piece>,
    cursor: Position,
    selection_anchor: Option<Position>,
}

/// Multiline editor buffer with embedded [`Atom`]s and undo/redo.
#[derive(Debug, Clone)]
pub struct TextBuffer {
    pieces: Vec<Piece>,
    cursor: Position,
    selection_anchor: Option<Position>,
    revision: u64,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    paste_line_threshold: u32,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            pieces: vec![Piece::Text(String::new())],
            cursor: 0,
            selection_anchor: None,
            revision: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            paste_line_threshold: 8,
        }
    }

    pub fn with_paste_line_threshold(mut self, lines: u32) -> Self {
        self.paste_line_threshold = lines;
        self
    }

    pub fn cursor(&self) -> Position {
        self.cursor
    }

    pub fn selection(&self) -> Option<Range<Position>> {
        self.selection_anchor.map(|anchor| {
            if anchor <= self.cursor {
                anchor..self.cursor
            } else {
                self.cursor..anchor
            }
        })
    }

    pub fn set_cursor(&mut self, pos: Position) {
        self.cursor = pos.clamp(0, self.display_len());
        self.selection_anchor = None;
        self.bump_revision();
    }

    pub fn text(&self) -> String {
        self.display()
    }

    pub fn display(&self) -> String {
        let mut out = String::new();
        for piece in &self.pieces {
            match piece {
                Piece::Text(s) => out.push_str(s),
                Piece::Atom(a) => out.push_str(&a.display()),
            }
        }
        out
    }

    pub fn display_len(&self) -> Position {
        self.pieces.iter().map(Piece::display_len).sum()
    }

    pub fn set_text(&mut self, s: &str) {
        self.save_undo();
        self.pieces = vec![Piece::Text(s.to_owned())];
        self.cursor = s.len();
        self.selection_anchor = None;
        self.bump_revision();
    }

    pub fn insert_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        if self.selection().is_none() {
            self.save_undo();
        } else {
            self.delete_selection();
        }
        let pos = self.cursor;
        self.insert_str_at(pos, s);
        self.cursor = pos + s.len();
        self.bump_revision();
    }

    pub fn insert_atom(&mut self, atom: Atom) {
        if self.selection().is_none() {
            self.save_undo();
        } else {
            self.delete_selection();
        }
        let pos = self.cursor;
        let len = atom.display().len();
        self.insert_piece_at(pos, Piece::Atom(atom));
        self.cursor = pos + len;
        self.bump_revision();
    }

    pub fn insert_paste(&mut self, text: &str) {
        let line_count = text.lines().count().max(1) as u32;
        if line_count >= self.paste_line_threshold {
            self.insert_atom(Atom::pasted_block(line_count));
        } else {
            self.insert_str(text);
        }
    }

    pub fn delete_backward(&mut self) {
        if self.delete_selection().is_some() {
            return;
        }
        if self.cursor == 0 {
            return;
        }
        self.save_undo();
        let new_cursor = self.prev_grapheme_boundary(self.cursor);
        self.delete_range(new_cursor, self.cursor);
        self.cursor = new_cursor;
        self.bump_revision();
    }

    pub fn delete_forward(&mut self) {
        if self.delete_selection().is_some() {
            return;
        }
        let len = self.display_len();
        if self.cursor >= len {
            return;
        }
        self.save_undo();
        let end = self.next_grapheme_boundary(self.cursor);
        self.delete_range(self.cursor, end);
        self.bump_revision();
    }

    pub fn move_left(&mut self) {
        self.cursor = self.prev_grapheme_boundary(self.cursor);
        self.selection_anchor = None;
    }

    pub fn move_right(&mut self) {
        self.cursor = self.next_grapheme_boundary(self.cursor);
        self.selection_anchor = None;
    }

    pub fn move_up(&mut self) {
        let (line, col) = self.line_col(self.cursor);
        if line == 0 {
            self.cursor = 0;
        } else {
            let prev_start = self.line_start(line - 1);
            let prev_len = self.line_len(line - 1);
            self.cursor = prev_start + col.min(prev_len);
        }
        self.selection_anchor = None;
    }

    pub fn move_down(&mut self) {
        let (line, col) = self.line_col(self.cursor);
        let lines = self.line_count();
        if line + 1 >= lines {
            self.cursor = self.display_len();
        } else {
            let next_start = self.line_start(line + 1);
            let next_len = self.line_len(line + 1);
            self.cursor = next_start + col.min(next_len);
        }
        self.selection_anchor = None;
    }

    pub fn undo(&mut self) -> bool {
        if let Some(snap) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.restore(snap);
            self.bump_revision();
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(snap) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.restore(snap);
            self.bump_revision();
            true
        } else {
            false
        }
    }

    pub fn atoms(&self) -> Vec<&Atom> {
        self.pieces.iter().filter_map(|p| p.as_atom()).collect()
    }

    fn line_col(&self, pos: Position) -> (usize, usize) {
        let text = self.display();
        let before = text.get(..pos).unwrap_or(&text);
        let line = before.matches('\n').count();
        let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = pos - line_start;
        (line, col)
    }

    fn line_start(&self, line: usize) -> Position {
        if line == 0 {
            return 0;
        }
        let text = self.display();
        let mut count = 0;
        for (idx, _) in text.match_indices('\n') {
            count += 1;
            if count == line {
                return idx + 1;
            }
        }
        self.display_len()
    }

    fn line_len(&self, line: usize) -> usize {
        let start = self.line_start(line);
        let text = self.display();
        let rest = text.get(start..).unwrap_or("");
        rest.find('\n').unwrap_or(rest.len())
    }

    fn line_count(&self) -> usize {
        self.display().matches('\n').count() + 1
    }

    fn delete_selection(&mut self) -> Option<Range<Position>> {
        let range = self.selection()?;
        self.save_undo();
        self.delete_range(range.start, range.end);
        self.cursor = range.start;
        self.selection_anchor = None;
        self.bump_revision();
        Some(range)
    }

    fn delete_range(&mut self, start: Position, end: Position) {
        if start >= end {
            return;
        }
        let mut rebuilt = Vec::new();
        let mut offset = 0;
        for piece in &self.pieces {
            let piece_start = offset;
            let piece_end = offset + piece.display_len();
            offset = piece_end;

            if piece_end <= start || piece_start >= end {
                rebuilt.push(piece.clone());
                continue;
            }

            match piece {
                Piece::Atom(_) => {
                    // Entire atom removed if range covers it; partial overlap keeps atom.
                    if start <= piece_start && end >= piece_end {
                        continue;
                    }
                    rebuilt.push(piece.clone());
                }
                Piece::Text(text) => {
                    let rel_start = start.saturating_sub(piece_start);
                    let rel_end = end.saturating_sub(piece_start).min(text.len());
                    let before = text.get(..rel_start).unwrap_or("");
                    let after = text.get(rel_end..).unwrap_or("");
                    let merged = format!("{before}{after}");
                    if !merged.is_empty() {
                        rebuilt.push(Piece::Text(merged));
                    }
                }
            }
        }
        self.pieces = coalesce_text(rebuilt);
    }

    fn insert_str_at(&mut self, pos: Position, s: &str) {
        let mut rebuilt = Vec::new();
        let mut offset = 0;
        let mut inserted = false;

        for piece in &self.pieces {
            let piece_start = offset;
            let piece_end = offset + piece.display_len();
            offset = piece_end;

            if inserted {
                rebuilt.push(piece.clone());
                continue;
            }

            if pos < piece_start || pos > piece_end {
                rebuilt.push(piece.clone());
                continue;
            }

            match piece {
                Piece::Atom(_) => {
                    if pos == piece_start {
                        rebuilt.push(Piece::Text(s.to_owned()));
                        rebuilt.push(piece.clone());
                    } else {
                        rebuilt.push(piece.clone());
                        rebuilt.push(Piece::Text(s.to_owned()));
                    }
                    inserted = true;
                }
                Piece::Text(text) => {
                    let rel = (pos - piece_start).min(text.len());
                    let (before, after) = text.split_at(rel);
                    if !before.is_empty() {
                        rebuilt.push(Piece::Text(before.to_owned()));
                    }
                    rebuilt.push(Piece::Text(s.to_owned()));
                    if !after.is_empty() {
                        rebuilt.push(Piece::Text(after.to_owned()));
                    }
                    inserted = true;
                }
            }
        }

        if !inserted {
            rebuilt.push(Piece::Text(s.to_owned()));
        }

        self.pieces = coalesce_text(rebuilt);
    }

    fn insert_piece_at(&mut self, pos: Position, piece: Piece) {
        let mut rebuilt = Vec::new();
        let mut offset = 0;
        let mut inserted = false;

        for existing in &self.pieces {
            let piece_start = offset;
            let piece_end = offset + existing.display_len();
            offset = piece_end;

            if inserted {
                rebuilt.push(existing.clone());
                continue;
            }

            if pos < piece_start || pos > piece_end {
                rebuilt.push(existing.clone());
                continue;
            }

            match existing {
                Piece::Atom(_) => {
                    if pos == piece_start {
                        rebuilt.push(piece.clone());
                        rebuilt.push(existing.clone());
                    } else {
                        rebuilt.push(existing.clone());
                        rebuilt.push(piece.clone());
                    }
                    inserted = true;
                }
                Piece::Text(text) => {
                    let rel = (pos - piece_start).min(text.len());
                    let (before, after) = text.split_at(rel);
                    if !before.is_empty() {
                        rebuilt.push(Piece::Text(before.to_owned()));
                    }
                    rebuilt.push(piece.clone());
                    if !after.is_empty() {
                        rebuilt.push(Piece::Text(after.to_owned()));
                    }
                    inserted = true;
                }
            }
        }

        if !inserted {
            rebuilt.push(piece);
        }

        self.pieces = coalesce_text(rebuilt);
    }

    fn prev_grapheme_boundary(&self, pos: Position) -> Position {
        if pos == 0 {
            return 0;
        }
        let text = self.display();
        let before = text.get(..pos).unwrap_or(text.as_str());
        let mut index = 0;
        let mut last = 0;
        for g in before.graphemes(true) {
            last = index;
            index += g.len();
        }
        if index == pos {
            last
        } else {
            pos.saturating_sub(1)
        }
    }

    fn next_grapheme_boundary(&self, pos: Position) -> Position {
        let text = self.display();
        let mut index = 0;
        for g in text.graphemes(true) {
            if index >= pos {
                return index + g.len();
            }
            index += g.len();
        }
        text.len()
    }

    fn save_undo(&mut self) {
        self.undo.push(self.snapshot());
        self.redo.clear();
        if self.undo.len() > 128 {
            self.undo.remove(0);
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            pieces: self.pieces.clone(),
            cursor: self.cursor,
            selection_anchor: self.selection_anchor,
        }
    }

    fn restore(&mut self, snap: Snapshot) {
        self.pieces = snap.pieces;
        self.cursor = snap.cursor;
        self.selection_anchor = snap.selection_anchor;
    }

    fn bump_revision(&mut self) {
        self.revision += 1;
    }
}

fn coalesce_text(pieces: Vec<Piece>) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    for piece in pieces {
        match piece {
            Piece::Text(text) if text.is_empty() => {}
            Piece::Text(text) => {
                if let Some(Piece::Text(prev)) = out.last_mut() {
                    prev.push_str(&text);
                } else {
                    out.push(Piece::Text(text));
                }
            }
            other => out.push(other),
        }
    }
    if out.is_empty() {
        out.push(Piece::Text(String::new()));
    }
    out
}

impl Revisioned for TextBuffer {
    fn revision(&self) -> u64 {
        self.revision
    }
}
