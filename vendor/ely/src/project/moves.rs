use gpui::Pixels;

/// Where a card whose middle sits at `middle` lands among rows of `heights` from a column's `top`: past every row whose middle is above it.
pub(crate) fn landing(heights: &[Pixels], top: Pixels, middle: Pixels) -> usize {
    let mut y = top;
    let mut index = 0;
    for height in heights {
        if y + *height / 2.0 < middle {
            index += 1;
        }
        y += *height;
    }
    index
}

/// Where Option and an arrow take the card at `index` of `column`, among columns of `lens` cards: up and down within it, left and right to the same place or the end.
pub(crate) fn stepped(
    lens: &[usize],
    column: usize,
    index: usize,
    key: &str,
) -> Option<(usize, usize)> {
    match key {
        "up" => (index > 0).then(|| (column, index - 1)),
        "down" => (index + 1 < lens[column]).then(|| (column, index + 1)),
        "left" => (column > 0).then(|| (column - 1, index.min(lens[column - 1]))),
        "right" => (column + 1 < lens.len()).then(|| (column + 1, index.min(lens[column + 1]))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::{landing, stepped};

    #[test]
    fn a_card_lands_past_the_rows_whose_middles_it_passed() {
        let rows = [px(40.0), px(60.0), px(40.0)];
        assert_eq!(landing(&rows, px(100.0), px(110.0)), 0);
        assert_eq!(landing(&rows, px(100.0), px(125.0)), 1);
        assert_eq!(landing(&rows, px(100.0), px(175.0)), 2);
        assert_eq!(landing(&rows, px(100.0), px(400.0)), 3);
    }

    #[test]
    fn option_arrows_step_within_and_across_columns() {
        let lens = [3, 0, 2];
        assert_eq!(stepped(&lens, 0, 0, "up"), None);
        assert_eq!(stepped(&lens, 0, 1, "down"), Some((0, 2)));
        assert_eq!(stepped(&lens, 0, 2, "down"), None);
        assert_eq!(stepped(&lens, 0, 2, "right"), Some((1, 0)));
        assert_eq!(stepped(&lens, 2, 1, "left"), Some((1, 0)));
        assert_eq!(stepped(&lens, 2, 1, "right"), None);
        assert_eq!(stepped(&lens, 1, 0, "home"), None);
        assert_eq!(
            stepped(&[3, 2], 0, 1, "right"),
            Some((1, 1)),
            "the same place"
        );
        assert_eq!(
            stepped(&[3, 2], 1, 1, "left"),
            Some((0, 1)),
            "the same place"
        );
        assert_eq!(
            stepped(&[3, 2], 0, 2, "right"),
            Some((1, 2)),
            "the end of a shorter column"
        );
    }
}
