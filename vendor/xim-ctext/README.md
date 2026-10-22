# XIM compound text decoding backport

`src/lib.rs` and `LICENSE` are copied unchanged from Riey/xim-rs commit
[`5e18bce574c5e5f1bae80471258192e74c67691e`](https://github.com/Riey/xim-rs/tree/5e18bce574c5e5f1bae80471258192e74c67691e/xim-ctext)
(xim-ctext 0.4.1, MIT). Cargo metadata retains version 0.3.0 so the local patch
satisfies zed-xim 0.4.0-zed's `^0.3.0` requirement. This is a local backport,
not the crates.io 0.3.0 release; Cargo.lock selects this path dependency.

The published 0.3.0 decoder rejects GB2312 COMPOUND_TEXT from Chinese X11
input methods. zed-xim calls `expect("Encoding Error")` on the result and
terminates the application. The upstream decoder supports CJK encodings and
switching back to ASCII, including mixed Chinese/date commits. Its public
API remains compatible with this caller. No application source is changed.

The root Linux `tests/ime.rs` exercises this same resolved dependency, and
`cargo test -p xim-ctext --locked` runs its upstream tests. Keep these tests
when updating GPUI; remove this patch once its dependency resolves a released
decoder with both CJK and ASCII-switch fixes. Do not edit Cargo's cache.

## Validation for ELYTerminal 0.1.8

Ubuntu 22.04, X11 and IBus 1.5.26: the original crash was reported in both
DEB and AppImage builds, with `Encoding Error: UnsupportedEncoding` in the
journal. A GB2312 decoder fixture reproduces the failure without claiming
to capture the input method's actual packet. The user confirmed Chinese input
works after the backport. This acceptance covers the tested desktop only.

Nine upstream decoder tests, four Linux IME integration tests and all 276
existing application tests pass, together with fmt and clippy. The hosted
Linux IME workflow repeats these checks as a regular user in a clean container.
Wayland and other input methods have not been tested for this fix.
