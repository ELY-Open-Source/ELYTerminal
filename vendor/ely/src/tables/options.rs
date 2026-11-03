use std::rc::Rc;

use gpui::{AnyElement, App, ElementId, SharedString, Window, div};

use super::{Column, FilterRule, Row, SortKey, table::DataTable};
use crate::theme::Density;

impl DataTable {
    pub fn new(id: impl Into<ElementId>, columns: impl IntoIterator<Item = Column>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            columns: columns.into_iter().collect(),
            rows: Rc::default(),
            query: SharedString::default(),
            filters: Vec::new(),
            any: false,
            sorts: Vec::new(),
            hidden: Vec::new(),
            page_size: None,
            virtualized: false,
            selected: None,
            on_select: None,
            detail: None,
            group_by: None,
            on_edit: None,
            density: Density::Standard,
        }
    }

    /// The rows, owned or shared; a shared list is not copied.
    pub fn rows(mut self, rows: impl Into<Rc<Vec<Row>>>) -> Self {
        self.rows = rows.into();
        self
    }

    /// Keeps the rows holding this text in any cell.
    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = query.into();
        self
    }

    /// Rules from a `FilterBuilder`: rows pass them all, or any when `any`.
    pub fn filters(mut self, rules: impl IntoIterator<Item = FilterRule>, any: bool) -> Self {
        self.filters = rules.into_iter().collect();
        self.any = any;
        self
    }

    /// Sort keys from the owner, such as a `SortBuilder`'s; header presses change them until the owner sends new ones.
    pub fn sorts(mut self, keys: impl IntoIterator<Item = SortKey>) -> Self {
        self.sorts = keys
            .into_iter()
            .map(|key| (key.column, key.rising))
            .collect();
        self
    }

    /// Columns left out, such as a toolbar's menu hides.
    pub fn hidden(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.hidden = keys.into_iter().map(Into::into).collect();
        self
    }

    /// Splits the rows into pages of `size`.
    pub fn paged(mut self, size: usize) -> Self {
        assert!(size > 0, "a page holds at least a row");
        self.page_size = Some(size);
        self
    }

    /// Draws only the rows in view, for long tables. Give it a height.
    pub fn virtualized(mut self) -> Self {
        self.virtualized = true;
        self
    }

    /// Shows a box on each row; `keys` are the rows selected.
    pub fn selected(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = Some(keys.into_iter().map(Into::into).collect());
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// A disclosure on each row opens what `detail` draws for it, under the row.
    pub fn detail(
        mut self,
        detail: impl Fn(&SharedString, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.detail = Some(Rc::new(detail));
        self
    }

    /// Gathers rows under their value in column `key`; each group folds.
    pub fn group_by(mut self, key: impl Into<SharedString>) -> Self {
        self.group_by = Some(key.into());
        self
    }

    /// Gets a row's key, a column's key and the new text, after a double press on an editable cell.
    pub fn on_edit(
        mut self,
        handler: impl Fn(&SharedString, &SharedString, &SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_edit = Some(Rc::new(handler));
        self
    }

    pub fn density(mut self, density: Density) -> Self {
        self.density = density;
        self
    }
}
