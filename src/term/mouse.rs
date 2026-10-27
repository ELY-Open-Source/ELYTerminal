/// A button as reports name it; the wheel's two turns count as buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
}

/// What the mouse did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    Press,
    Release,
    Motion,
}

/// A mouse event a program may ask for.
#[derive(Clone, Copy, Debug)]
pub struct Report {
    /// None for motion with no button down.
    pub button: Option<Button>,
    pub gesture: Gesture,
    /// Column and row on screen, from 0.
    pub cell: (usize, usize),
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
}

/// The mouse events a program asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tracking {
    #[default]
    Off,
    /// Mode 9: presses alone, no modifiers.
    X10,
    /// Mode 1000: presses and releases.
    Normal,
    /// Mode 1002: and motion with a button down.
    Button,
    /// Mode 1003: and all motion.
    Any,
}

/// The bytes reports go in: xterm's default, mode 1005 or mode 1006.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Format {
    #[default]
    X10,
    Utf8,
    Sgr,
}

/// How a program asked for the mouse.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseMode {
    pub tracking: Tracking,
    pub format: Format,
}

impl MouseMode {
    /// Follows `CSI ? mode h` or `l`; as in xterm, a reset ends the tracking or format.
    pub fn apply(&mut self, mode: u32, set: bool) {
        let tracking = match mode {
            9 => Tracking::X10,
            1000 => Tracking::Normal,
            1002 => Tracking::Button,
            1003 => Tracking::Any,
            1005 | 1006 => {
                self.format = match (set, mode) {
                    (false, _) => Format::X10,
                    (true, 1005) => Format::Utf8,
                    (true, _) => Format::Sgr,
                };
                return;
            }
            _ => return,
        };
        self.tracking = if set { tracking } else { Tracking::Off };
    }
}

/// What a wheel event sends: the tracking and format the reports go in.
pub(super) fn asked(hidden: bool, tracking: Tracking, format: Format) -> (Tracking, Format) {
    if hidden {
        (Tracking::Button, Format::Sgr)
    } else {
        (tracking, format)
    }
}

/// The bytes `report` sends; none when unasked for or past what the format holds.
pub fn encode(report: &Report, mode: MouseMode) -> Option<Vec<u8>> {
    let clicked = matches!(
        report.button,
        Some(Button::Left | Button::Middle | Button::Right)
    );
    let wanted = match mode.tracking {
        Tracking::Off => false,
        Tracking::X10 => report.gesture == Gesture::Press && clicked,
        Tracking::Normal => report.gesture != Gesture::Motion,
        Tracking::Button => report.gesture != Gesture::Motion || report.button.is_some(),
        Tracking::Any => true,
    };
    if !wanted {
        return None;
    }
    let sgr = mode.format == Format::Sgr;
    let mut code = match report.button {
        None => 3,
        Some(_) if report.gesture == Gesture::Release && !sgr => 3,
        Some(Button::Left) => 0,
        Some(Button::Middle) => 1,
        Some(Button::Right) => 2,
        Some(Button::WheelUp) => 64,
        Some(Button::WheelDown) => 65,
    };
    if mode.tracking != Tracking::X10 {
        code +=
            4 * u32::from(report.shift) + 8 * u32::from(report.alt) + 16 * u32::from(report.ctrl);
    }
    if report.gesture == Gesture::Motion {
        code += 32;
    }
    let (column, row) = (report.cell.0 + 1, report.cell.1 + 1);
    if sgr {
        let end = if report.gesture == Gesture::Release {
            'm'
        } else {
            'M'
        };
        return Some(format!("\x1b[<{code};{column};{row}{end}").into_bytes());
    }
    let utf8 = mode.format == Format::Utf8;
    let most = if utf8 { 0x7FF - 32 } else { 0xFF - 32 };
    if column > most || row > most {
        log::debug!("mouse: cell {column},{row} is past what the format holds");
        return None;
    }
    let mut bytes = b"\x1b[M".to_vec();
    bytes.push(32 + code as u8);
    for value in [column, row] {
        let value = 32 + value as u32;
        if utf8 {
            let ch = char::from_u32(value).expect("a code point under U+0800");
            bytes.extend(ch.to_string().as_bytes());
        } else {
            bytes.push(value as u8);
        }
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(button: Option<Button>, gesture: Gesture, cell: (usize, usize)) -> Report {
        Report {
            button,
            gesture,
            cell,
            shift: false,
            alt: false,
            ctrl: false,
        }
    }

    fn mode(tracking: Tracking, format: Format) -> MouseMode {
        MouseMode { tracking, format }
    }

    const LEFT: Option<Button> = Some(Button::Left);

    #[test]
    fn one_tracking_and_one_format_hold_and_a_reset_ends_them() {
        let mut held = MouseMode::default();
        for (code, set) in [(1000, true), (9, true), (9, false), (1006, true)] {
            held.apply(code, set);
        }
        assert_eq!(held, mode(Tracking::Off, Format::Sgr), "9 replaced 1000");
        held.apply(1002, true);
        held.apply(1003, false);
        assert_eq!(held.tracking, Tracking::Off, "any reset ends tracking");
        held.apply(1003, true);
        held.apply(1005, true);
        assert_eq!(
            held,
            mode(Tracking::Any, Format::Utf8),
            "the last format holds"
        );
        held.apply(1006, false);
        held.apply(25, true);
        assert_eq!(held, mode(Tracking::Any, Format::X10));
    }

    #[test]
    fn normal_tracking_sends_presses_and_releases_as_x10_bytes() {
        let normal = mode(Tracking::Normal, Format::X10);
        let press = report(LEFT, Gesture::Press, (0, 0));
        assert_eq!(encode(&press, normal), Some(b"\x1b[M !!".to_vec()));
        let release = report(LEFT, Gesture::Release, (3, 2));
        assert_eq!(encode(&release, normal), Some(b"\x1b[M#$#".to_vec()));
        let motion = report(LEFT, Gesture::Motion, (1, 1));
        assert_eq!(encode(&motion, normal), None);
    }

    #[test]
    fn sgr_names_the_released_button_and_the_modifiers() {
        let sgr = mode(Tracking::Normal, Format::Sgr);
        let press = Report {
            shift: true,
            ctrl: true,
            ..report(Some(Button::Right), Gesture::Press, (4, 2))
        };
        assert_eq!(encode(&press, sgr), Some(b"\x1b[<22;5;3M".to_vec()));
        let release = report(Some(Button::Middle), Gesture::Release, (300, 9));
        assert_eq!(encode(&release, sgr), Some(b"\x1b[<1;301;10m".to_vec()));
        let alt = Report {
            alt: true,
            ..report(Some(Button::WheelDown), Gesture::Press, (0, 0))
        };
        assert_eq!(encode(&alt, sgr), Some(b"\x1b[<73;1;1M".to_vec()));
    }

    #[test]
    fn motion_needs_button_or_any_tracking() {
        let held = report(LEFT, Gesture::Motion, (1, 0));
        let free = report(None, Gesture::Motion, (1, 0));
        let button = mode(Tracking::Button, Format::Sgr);
        assert_eq!(encode(&held, button), Some(b"\x1b[<32;2;1M".to_vec()));
        assert_eq!(encode(&free, button), None);
        let any = mode(Tracking::Any, Format::Sgr);
        assert_eq!(encode(&free, any), Some(b"\x1b[<35;2;1M".to_vec()));
    }

    #[test]
    fn x10_tracking_sends_presses_alone_without_modifiers() {
        let x10 = mode(Tracking::X10, Format::X10);
        let press = Report {
            shift: true,
            ..report(LEFT, Gesture::Press, (0, 0))
        };
        assert_eq!(encode(&press, x10), Some(b"\x1b[M !!".to_vec()));
        let release = report(LEFT, Gesture::Release, (0, 0));
        assert_eq!(encode(&release, x10), None);
        let wheel = report(Some(Button::WheelUp), Gesture::Press, (0, 0));
        assert_eq!(encode(&wheel, x10), None);
        assert_eq!(encode(&press, MouseMode::default()), None);
    }

    #[test]
    fn cells_past_the_format_go_utf8_or_nowhere() {
        let normal = mode(Tracking::Normal, Format::X10);
        let edge = report(LEFT, Gesture::Press, (222, 0));
        assert_eq!(
            encode(&edge, normal),
            Some(vec![0x1b, b'[', b'M', 32, 255, 33])
        );
        let past = report(LEFT, Gesture::Press, (223, 0));
        assert_eq!(encode(&past, normal), None);
        let utf8 = mode(Tracking::Normal, Format::Utf8);
        let mut wide = b"\x1b[M ".to_vec();
        wide.extend("\u{100}!".as_bytes());
        assert_eq!(encode(&past, utf8), Some(wide));
        let far = report(LEFT, Gesture::Press, (2015, 0));
        assert_eq!(encode(&far, utf8), None);
    }

    #[test]
    fn a_hidden_mouse_mode_asks_for_the_wheel_in_sgr() {
        // ConPTY drops a program's DECSET, so the tap never learned it: the wheel goes as an SGR report, which ConPTY translates for the program.
        assert_eq!(
            asked(true, Tracking::Off, Format::X10),
            (Tracking::Button, Format::Sgr)
        );
        // A mode the tap did read holds, whatever it asked for.
        let seen = mode(Tracking::Normal, Format::Utf8);
        assert_eq!(
            asked(false, seen.tracking, seen.format),
            (seen.tracking, seen.format)
        );
        // Shift holds the mouse for selecting, so the wheel stays the terminal's.
        assert_eq!(
            asked(false, Tracking::Off, Format::Sgr),
            (Tracking::Off, Format::Sgr)
        );
    }
}
