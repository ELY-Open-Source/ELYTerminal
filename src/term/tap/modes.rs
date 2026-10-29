//! The private modes the tap follows before the parser.

use super::PARAMS;
use crate::term::mouse::MouseMode;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Csi {
    #[default]
    Ground,
    Escape,
    Bracket,
    Private,
}

/// The mouse modes as xterm holds them, read as vte reads; alacritty drops 9. On Windows, win32-input-mode too.
#[derive(Default)]
pub(crate) struct Modes {
    state: Csi,
    params: Vec<u32>,
    /// Values begun, subparameters too.
    fields: usize,
    /// Past a `:`, whose subparameters the parser's mode sets skip.
    sub: bool,
    pub(super) mode: MouseMode,
    /// Keys go as Windows' records, as ConPTY asks; a full reset ends it, as in Windows Terminal.
    #[cfg(windows)]
    pub(super) win32: bool,
}

impl Modes {
    /// The mode after `bytes`, when they changed it.
    pub fn feed(&mut self, bytes: &[u8]) -> Option<MouseMode> {
        let was = self.mode;
        let mut rest = bytes;
        while !rest.is_empty() {
            if self.state == Csi::Ground {
                let Some(at) = memchr::memchr(0x1b, rest) else {
                    break;
                };
                self.state = Csi::Escape;
                rest = &rest[at + 1..];
                continue;
            }
            let byte = rest[0];
            rest = &rest[1..];
            self.state = match (self.state, byte) {
                (_, 0x18 | 0x1a) => Csi::Ground,
                (_, 0x1b) => Csi::Escape,
                // Mid-sequence, C0 runs and DEL is skipped, as in vte.
                (state, 0x00..=0x1f | 0x7f..=0xff) => state,
                (Csi::Escape, b'[') => Csi::Bracket,
                (Csi::Escape, b'c') => {
                    self.mode = MouseMode::default();
                    #[cfg(windows)]
                    {
                        self.win32 = false;
                    }
                    Csi::Ground
                }
                (Csi::Bracket, b'?') => {
                    self.params = vec![0];
                    self.fields = 1;
                    self.sub = false;
                    Csi::Private
                }
                (Csi::Private, b'0'..=b'9') => {
                    if !self.sub {
                        let last = self.params.last_mut().expect("a parameter in progress");
                        *last = last
                            .saturating_mul(10)
                            .saturating_add(u32::from(byte - b'0'));
                    }
                    Csi::Private
                }
                (Csi::Private, b':' | b';') if self.fields < PARAMS => {
                    self.fields += 1;
                    self.sub = byte == b':';
                    if !self.sub {
                        self.params.push(0);
                    }
                    Csi::Private
                }
                (Csi::Private, b'h' | b'l') => {
                    for &mode in &self.params {
                        self.mode.apply(mode, byte == b'h');
                        #[cfg(windows)]
                        if mode == crate::term::win32::MODE {
                            self.win32 = byte == b'h';
                        }
                    }
                    Csi::Ground
                }
                _ => Csi::Ground,
            };
        }
        (self.mode != was).then_some(self.mode)
    }
}
