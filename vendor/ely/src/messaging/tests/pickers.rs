use std::path::Path;

use gpui::{
    Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, TestAppContext,
    VisualTestContext, Window, div, px,
};

use super::{press, settle, setup, tab_to};
use crate::{
    forms,
    messaging::{Gif, GifPicker, Gifs, Sticker, StickerPack, StickerPicker},
};

fn sticker(key: &'static str, name: &'static str) -> Sticker {
    Sticker::new(key, name, Path::new("/nowhere.png"))
}

fn packs() -> [StickerPack; 2] {
    [
        StickerPack::new(
            "Atrium",
            [
                sticker("arch", "Arch"),
                sticker("stair", "Stair"),
                sticker("column", "Column"),
            ],
        ),
        StickerPack::new(
            "Garden",
            [sticker("olive", "Olive"), sticker("stone", "Stone")],
        ),
    ]
}

/// A picker and what it picked.
struct Stickers {
    picked: Vec<SharedString>,
}

impl Render for Stickers {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        StickerPicker::new("stickers", packs())
            .on_pick(move |key, _, cx| owner.update(cx, |host, _| host.picked.push(key.clone())))
    }
}

fn stickering(cx: &mut TestAppContext) -> (Entity<Stickers>, &mut VisualTestContext) {
    setup(cx);
    cx.update(forms::bind_keys);
    let (host, cx) = cx.add_window_view(|_, _| Stickers { picked: Vec::new() });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (host, cx)
}

fn picked(host: &Entity<Stickers>, cx: &mut VisualTestContext) -> Vec<SharedString> {
    host.read_with(cx, |host, _| host.picked.clone())
}

#[gpui::test]
fn a_search_spans_every_pack(cx: &mut TestAppContext) {
    let (host, cx) = stickering(cx);
    tab_to(3, cx);
    cx.simulate_input("st");
    settle(cx);
    cx.simulate_keystrokes("down right enter");
    settle(cx);
    assert_eq!(
        picked(&host, cx),
        ["stone"],
        "Stair and Stone sit side by side, a pack apart"
    );
}

#[gpui::test]
fn the_chosen_pack_fills_the_grid(cx: &mut TestAppContext) {
    let (host, cx) = stickering(cx);
    tab_to(4, cx);
    press("enter", cx);
    tab_to(2, cx);
    press("space", cx);
    tab_to(4, cx);
    press("enter", cx);
    assert_eq!(picked(&host, cx), ["arch", "olive"]);
}

#[test]
#[should_panic(expected = "pack Atrium twice")]
fn a_pack_is_named_once() {
    let _ = StickerPicker::new(
        "stickers",
        [
            StickerPack::new("Atrium", [sticker("arch", "Arch")]),
            StickerPack::new("Atrium", [sticker("stair", "Stair")]),
        ],
    );
}

#[test]
#[should_panic(expected = "sticker arch twice")]
fn a_sticker_is_listed_once() {
    let _ = StickerPicker::new(
        "stickers",
        [
            StickerPack::new("A", [sticker("arch", "Arch")]),
            StickerPack::new("B", [sticker("arch", "Arch again")]),
        ],
    );
}

fn gif(key: &'static str) -> Gif {
    Gif::new(key, key, Path::new("/nowhere.gif"), 1.5)
}

/// A GIF picker showing `gifs`, and what it heard.
struct Gifing {
    gifs: Gifs,
    heard: Vec<String>,
}

impl Render for Gifing {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (query, pick) = (cx.entity(), cx.entity());
        div().w(px(360.0)).child(
            GifPicker::new("gifs", self.gifs.clone())
                .categories(["Dunes", "Stairs"])
                .on_query(move |words, _, cx| {
                    query.update(cx, |host, _| host.heard.push(format!("query {words}")))
                })
                .on_pick(move |key, _, cx| {
                    pick.update(cx, |host, _| host.heard.push(format!("pick {key}")))
                }),
        )
    }
}

fn gifing(gifs: Gifs, cx: &mut TestAppContext) -> (Entity<Gifing>, &mut VisualTestContext) {
    setup(cx);
    cx.update(forms::bind_keys);
    let (host, cx) = cx.add_window_view(|_, _| Gifing {
        gifs,
        heard: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (host, cx)
}

fn heard(host: &Entity<Gifing>, cx: &mut VisualTestContext) -> Vec<String> {
    host.read_with(cx, |host, _| host.heard.clone())
}

#[gpui::test]
fn typed_words_and_a_category_reach_the_host(cx: &mut TestAppContext) {
    let (host, cx) = gifing(Gifs::Found(vec![gif("a")]), cx);
    tab_to(3, cx);
    press("space", cx);
    tab_to(1, cx);
    cx.simulate_input(" more");
    settle(cx);
    assert_eq!(
        heard(&host, cx),
        [
            "query Stairs",
            "query Stairs m",
            "query Stairs mo",
            "query Stairs mor",
            "query Stairs more"
        ],
        "the second category fills the field, typing goes on from it, and a space alone asks nothing"
    );
}

#[gpui::test]
fn a_gif_is_a_tab_stop_that_space_picks(cx: &mut TestAppContext) {
    let (host, cx) = gifing(Gifs::Found(vec![gif("a"), gif("b")]), cx);
    tab_to(5, cx);
    press("space", cx);
    assert_eq!(heard(&host, cx), ["pick b"]);
}

#[gpui::test]
fn a_gif_past_the_fold_comes_into_view_when_focused(cx: &mut TestAppContext) {
    let keys = [
        "g0", "g1", "g2", "g3", "g4", "g5", "g6", "g7", "g8", "g9", "g10", "g11",
    ];
    let (_, cx) = gifing(Gifs::Found(keys.iter().map(|key| gif(key)).collect()), cx);
    tab_to(15, cx);
    let (tile, results) = (
        cx.debug_bounds("gif g11").expect("the last GIF draws"),
        cx.debug_bounds("gif-results").expect("the results draw"),
    );
    assert!(
        tile.top() >= results.top() && tile.bottom() <= results.bottom(),
        "{tile:?} inside {results:?}"
    );
}

#[gpui::test]
fn nothing_found_says_so(cx: &mut TestAppContext) {
    let (_, cx) = gifing(Gifs::Found(Vec::new()), cx);
    assert!(cx.debug_bounds("gifs-none").is_some());
}

#[gpui::test]
fn a_search_under_way_waits_without_saying_nothing(cx: &mut TestAppContext) {
    let (_, cx) = gifing(Gifs::Loading, cx);
    assert!(cx.debug_bounds("gif-picker").is_some());
    assert!(cx.debug_bounds("gifs-none").is_none());
}

#[test]
#[should_panic(expected = "gif a twice")]
fn a_gif_is_listed_once() {
    let _ = GifPicker::new("gifs", Gifs::Found(vec![gif("a"), gif("a")]));
}
