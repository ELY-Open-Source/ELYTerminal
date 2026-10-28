# xim-ctext

ELYTerminal's compound text decoder, in the place of crates.io's xim-ctext.

gpui's X11 input method client, zed-xim 0.4.0-zed, asks for `xim-ctext` `^0.3` and calls `expect` on each decode, so a decode error stops the app. crates.io's 0.3.0 refuses GB2312. xim-rs's newer decoder stops at JIS X 0212 and types Big5 and GBK segments as text. IBus and fcitx5 write all three for Chinese.

This crate keeps the name, the version and the one function zed-xim's client calls, `compound_text_to_utf8`; `[patch.crates-io]` in ELYTerminal's `Cargo.toml` takes it. It reads what libX11 writes, as ibus-x11 runs it in the C locale and as fcitx5's xcb-imdkit carries its code, and reads each set through libX11's own table. The web's tables, encoding_rs's among them, differ: Big5-HKSCS in 3,043 characters, and Big5, GB2312 and JIS X 0208 in a few, `・` and `☉` among them.

`tables/generate.sh` writes the tables from the lcUniConv of libX11 1.8.12, with `tables/generate.c` calling each set's own decoder; `tables/LICENSE` is libX11's notice for them. `tests/ime.rs` holds bytes libX11 wrote.

Remove the crate once gpui's zed-xim takes a released decoder that reads them as libX11 does.
