//! Wrapped text of the text field: shaping, visual rows, and the conversions between byte
//! offsets and points. GPUI shapes one `WrappedLine` per logical line; a row is one visual line.

use std::ops::Range;

use twake_gpui::{
    Font, Hsla, Pixels, Point, SharedString, TextRun, UnderlineStyle, Window, WrappedLine, point,
    px,
};

/// What the text is drawn with.
#[derive(Clone)]
pub(super) struct Face {
    pub font: Font,
    pub size: Pixels,
    pub color: Hsla,
    pub line_height: Pixels,
}

/// One visual line, in absolute byte offsets of the text.
pub(super) struct Row {
    line: usize,
    pub top: Pixels,
    pub start: usize,
    pub end: usize,
    line_start: usize,
    /// The last row of its logical line (the one that ends on a `'\n'` or on the text end).
    pub last: bool,
}

/// The shaped text with its rows.
pub(super) struct Laid {
    lines: Vec<WrappedLine>,
    line_height: Pixels,
    pub rows: Vec<Row>,
}

/// The runs of a text: the marked part is underlined.
fn runs(len: usize, marked: Option<&Range<usize>>, face: &Face) -> Vec<TextRun> {
    let run = |len: usize, underline: Option<UnderlineStyle>| TextRun {
        len,
        font: face.font.clone(),
        color: face.color,
        background_color: None,
        underline,
        strikethrough: None,
    };
    let Some(marked) = marked else {
        return vec![run(len, None)];
    };
    let underline = UnderlineStyle {
        color: Some(face.color),
        thickness: px(1.0),
        wavy: false,
    };
    vec![
        run(marked.start, None),
        run(marked.end - marked.start, Some(underline)),
        run(len - marked.end, None),
    ]
}

/// Shapes `text`, wrapped at `width` when given.
pub(super) fn shape(
    window: &Window,
    text: &SharedString,
    marked: Option<&Range<usize>>,
    face: &Face,
    width: Option<Pixels>,
) -> Vec<WrappedLine> {
    window
        .text_system()
        .shape_text(
            text.clone(),
            face.size,
            &runs(text.len(), marked, face),
            width,
            None,
        )
        .map(|lines| lines.into_iter().collect())
        .unwrap_or_default()
}

impl Laid {
    pub fn new(lines: Vec<WrappedLine>, line_height: Pixels) -> Self {
        let mut rows = Vec::new();
        let mut line_start = 0;
        let mut top = Pixels::ZERO;
        for (line, shaped) in lines.iter().enumerate() {
            let wraps: Vec<usize> = shaped
                .wrap_boundaries()
                .iter()
                .filter_map(|b| shaped.runs().get(b.run_ix)?.glyphs.get(b.glyph_ix))
                .map(|glyph| glyph.index)
                .collect();
            let mut start = 0;
            for (n, end) in wraps.iter().copied().chain([shaped.len()]).enumerate() {
                rows.push(Row {
                    line,
                    top,
                    start: line_start + start,
                    end: line_start + end,
                    line_start,
                    last: n == wraps.len(),
                });
                top += line_height;
                start = end;
            }
            line_start += shaped.len() + 1;
        }
        Self {
            lines,
            line_height,
            rows,
        }
    }

    /// Height of all the rows.
    pub fn height(&self) -> Pixels {
        self.line_height * self.rows.len().max(1) as f32
    }

    /// Distance from the start of `row` to the byte `offset` of its line.
    fn x_in(&self, row: &Row, offset: usize) -> Pixels {
        let Some(line) = self.lines.get(row.line) else {
            return Pixels::ZERO;
        };
        let layout = &line.unwrapped_layout;
        layout.x_for_index(offset - row.line_start) - layout.x_for_index(row.start - row.line_start)
    }

    /// The row holding `offset`: the last one that starts at or before it.
    pub fn row_of(&self, offset: usize) -> Option<&Row> {
        self.rows.iter().rev().find(|row| row.start <= offset)
    }

    /// The x of `offset` inside its row.
    pub fn x_of(&self, row: &Row, offset: usize) -> Pixels {
        self.x_in(row, offset.clamp(row.start, row.end))
    }

    /// Where the caret at `offset` stands, relative to the top left of the text.
    pub fn point_for_offset(&self, offset: usize) -> Point<Pixels> {
        self.row_of(offset).map_or_else(
            || point(Pixels::ZERO, Pixels::ZERO),
            |row| point(self.x_of(row, offset), row.top),
        )
    }

    /// The offset nearest to a point relative to the top left of the text.
    pub fn offset_for_point(&self, at: Point<Pixels>) -> usize {
        let Some(row) = self
            .rows
            .iter()
            .find(|row| at.y < row.top + self.line_height)
            .or(self.rows.last())
        else {
            return 0;
        };
        let Some(line) = self.lines.get(row.line) else {
            return 0;
        };
        let layout = &line.unwrapped_layout;
        let x = layout.x_for_index(row.start - row.line_start) + at.x;
        let (from, to) = (row.start - row.line_start, row.end - row.line_start);
        let mut index = layout.closest_index_for_x(x).clamp(from, to);
        if !row.last && index == to {
            // The end of a wrapped row is the start of the next one: stay on this row.
            index = line.text[..index]
                .char_indices()
                .next_back()
                .map_or(index, |(i, _)| i);
        }
        row.line_start + index
    }

    /// The top of the shaped line `line`.
    pub fn line_top(&self, line: usize) -> Pixels {
        self.rows
            .iter()
            .find(|row| row.line == line)
            .map_or(Pixels::ZERO, |row| row.top)
    }

    /// The shaped logical lines.
    pub fn lines(&self) -> &[WrappedLine] {
        &self.lines
    }
}

/// The scroll offset that shows the caret row, kept inside the content.
pub(super) fn scroll_to_caret(
    scroll: f32,
    caret_top: f32,
    row: f32,
    view: f32,
    content: f32,
) -> f32 {
    let wanted = if caret_top < scroll {
        caret_top
    } else if caret_top + row > scroll + view {
        caret_top + row - view
    } else {
        scroll
    };
    wanted.clamp(0.0, (content - view).max(0.0))
}
