/// The keys whose value is one letter, as in kitty.
const LETTERS: &[u8] = b"adeot";
/// The keys whose value may be negative.
const SIGNED: &[u8] = b"zHV";
/// The keys kitty knows; any other passes by.
const KNOWN: &[u8] = b"adeotfsvSOiIpmNqxywhXYcrCUzPQHV";

/// Which replies `q` keeps back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Quiet {
    No,
    Ok,
    Failures,
}

/// One graphics command: its control keys and its decoded payload.
#[derive(Debug)]
pub(super) struct Command {
    pub action: u8,
    pub quiet: Quiet,
    keys: [Option<u32>; 128],
    pub payload: Vec<u8>,
}

impl Command {
    /// The text after `G`: control keys, `;`, then base64.
    pub fn parse(text: &[u8]) -> Result<Self, String> {
        let (control, payload) = match text.iter().position(|&byte| byte == b';') {
            Some(at) => (&text[..at], &text[at + 1..]),
            None => (text, &b""[..]),
        };
        let mut keys = [None; 128];
        if !control.is_empty() {
            for pair in control.split(|&byte| byte == b',') {
                let [key, b'=', value @ ..] = pair else {
                    return Err(format!("{:?} is no key=value", lossy(pair)));
                };
                if !KNOWN.contains(key) {
                    continue;
                }
                keys[usize::from(*key)] = Some(read(*key, value)?);
            }
        }
        let quiet = match keys[usize::from(b'q')] {
            None | Some(0) => Quiet::No,
            Some(1) => Quiet::Ok,
            Some(_) => Quiet::Failures,
        };
        let payload = decode(payload)?;
        let action = keys[usize::from(b'a')].map_or(b't', |letter| letter as u8);
        Ok(Self {
            action,
            quiet,
            keys,
            payload,
        })
    }

    /// A number key's value, 0 when missing, as kitty defaults.
    pub fn int(&self, key: u8) -> u32 {
        self.keys[usize::from(key)].unwrap_or(0)
    }

    /// A letter key's value.
    pub fn letter(&self, key: u8) -> Option<u8> {
        self.keys[usize::from(key)].map(|letter| letter as u8)
    }

    /// Whether more chunks follow; only a direct payload comes so.
    pub fn more(&self) -> bool {
        self.letter(b't').is_none_or(|medium| medium == b'd') && self.int(b'm') > 0
    }
}

fn read(key: u8, value: &[u8]) -> Result<u32, String> {
    let fails = || {
        format!(
            "{} has no value of its type: {:?}",
            key as char,
            lossy(value)
        )
    };
    if LETTERS.contains(&key) {
        return match value {
            [letter] if letter.is_ascii_alphabetic() => Ok(u32::from(*letter)),
            _ => Err(fails()),
        };
    }
    let text = std::str::from_utf8(value).map_err(|_| fails())?;
    if SIGNED.contains(&key) {
        text.parse::<i32>()
            .map(|value| value as u32)
            .map_err(|_| fails())
    } else {
        text.parse::<u32>().map_err(|_| fails())
    }
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Base64 through SIMD, its padding optional, as kitty takes it.
pub(super) fn decode(text: &[u8]) -> Result<Vec<u8>, String> {
    let pad = text.iter().rev().take_while(|&&byte| byte == b'=').count();
    let bare = &text[..text.len() - pad];
    // Padding only finishes a partial group, as base64's Indifferent.
    if pad > 0 && (bare.len() % 4 < 2 || bare.len() % 4 + pad > 4) {
        return Err(format!("the payload's {pad} padding fits no group"));
    }
    base64_simd::STANDARD_NO_PAD
        .decode_to_vec(bare)
        .map_err(|error| format!("the payload is no base64: {error}"))
}
