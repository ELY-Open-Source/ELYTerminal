use std::{collections::HashMap, rc::Rc};

use gpui::{
    App, Div, ElementId, IntoElement, RenderOnce, SharedString, StyleRefinement, Styled, Window,
    div,
};

use super::{
    formula::{letters, shown},
    grid::{Face, Merge, OnCells, Text, grid},
};

/// Cells to edit as a spreadsheet's are, under column titles. Arrows walk and Shift stretches a range; typing or a double press edits; Delete clears; Cmd-C and Cmd-V copy and paste. `on_change` gets the edits and the owner keeps the text. Give it a height.
#[derive(IntoElement)]
pub struct DataGrid {
    id: ElementId,
    base: Div,
    titles: Vec<SharedString>,
    rows: Rc<Vec<Vec<SharedString>>>,
    on_change: Option<OnCells>,
}

impl DataGrid {
    pub fn new(
        id: impl Into<ElementId>,
        titles: impl IntoIterator<Item = impl Into<SharedString>>,
        rows: impl Into<Rc<Vec<Vec<SharedString>>>>,
    ) -> Self {
        Self {
            id: id.into(),
            base: div(),
            titles: titles.into_iter().map(Into::into).collect(),
            rows: rows.into(),
            on_change: None,
        }
    }

    /// Gets each edit as a row, a column and the new text.
    pub fn on_change(
        mut self,
        handler: impl Fn(&[(usize, usize, SharedString)], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Styled for DataGrid {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for DataGrid {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let cols = self.titles.len();
        for (ix, row) in self.rows.iter().enumerate() {
            assert_eq!(row.len(), cols, "row {ix} needs a cell per title");
        }
        let (rows, titles) = (self.rows.clone(), Rc::new(self.titles));
        let text: Text = Rc::new(move |row, col| rows[row][col].clone());
        let face = Face {
            id: self.id,
            rows: self.rows.len(),
            cols,
            titles: Rc::new(move |col| titles[col].clone()),
            numbered: false,
            raw: text.clone(),
            shown: text,
            merges: Vec::new(),
            frozen: (0, 0),
            fills: false,
            on_change: self.on_change,
        };
        grid(face, self.base, window, cx)
    }
}

/// A spreadsheet: lettered columns, numbered rows, formulas such as =SUM(A1:A5) or =B2*1.2, merged cells, frozen rows and columns, and a fill handle that carries a run on. `on_change` gets the edits and the owner keeps the cells. Give it a height.
#[derive(IntoElement)]
pub struct Spreadsheet {
    id: ElementId,
    base: Div,
    size: (usize, usize),
    cells: Rc<HashMap<(usize, usize), SharedString>>,
    merges: Vec<Merge>,
    frozen: (usize, usize),
    on_change: Option<OnCells>,
}

impl Spreadsheet {
    pub fn new(id: impl Into<ElementId>, rows: usize, cols: usize) -> Self {
        Self {
            id: id.into(),
            base: div(),
            size: (rows, cols),
            cells: Rc::default(),
            merges: Vec::new(),
            frozen: (0, 0),
            on_change: None,
        }
    }

    /// The cells by row and column from zero; text, numbers, or formulas that start with =.
    pub fn cells(mut self, cells: impl Into<Rc<HashMap<(usize, usize), SharedString>>>) -> Self {
        self.cells = cells.into();
        self
    }

    /// Shows a rectangle of cells as one, holding its first cell's text.
    pub fn merge(mut self, from: (usize, usize), to: (usize, usize)) -> Self {
        assert!(
            from.0 <= to.0 && from.1 <= to.1,
            "a merge runs from its top-left to its bottom-right"
        );
        if to.0 >= self.size.0 || to.1 >= self.size.1 {
            log::error!("sheet: a merge to {to:?} leaves {:?}; skipped", self.size);
            return self;
        }
        self.merges.push(Merge { from, to });
        self
    }

    /// Holds the first `rows` and `cols` still while the rest scrolls.
    pub fn freeze(mut self, rows: usize, cols: usize) -> Self {
        self.frozen = (rows, cols);
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&[(usize, usize, SharedString)], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Styled for Spreadsheet {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Spreadsheet {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let cells = self.cells.clone();
        let raw: Text =
            Rc::new(move |row, col| cells.get(&(row, col)).cloned().unwrap_or_default());
        let lookup = raw.clone();
        let face = Face {
            id: self.id,
            rows: self.size.0,
            cols: self.size.1,
            titles: Rc::new(|col| letters(col).into()),
            numbered: true,
            raw,
            shown: Rc::new(move |row, col| shown(&*lookup, row, col)),
            merges: self.merges,
            frozen: self.frozen,
            fills: true,
            on_change: self.on_change,
        };
        grid(face, self.base, window, cx)
    }
}
