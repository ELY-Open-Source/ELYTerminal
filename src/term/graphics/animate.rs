//! `a=f`, `a=a`, `a=c` and `d=f`, as Ghostty takes them.

use super::{
    Error, Graphics, Ids, Reply,
    command::Command,
    frames::{Ask, Frame, GAP, State, compose, fill},
    load::Loading,
};

impl Graphics {
    /// The image `ids` names, and when it was stored.
    fn named(&self, ids: Ids) -> Result<(u32, u64), Error> {
        let mut shared = self.lock();
        let store = &shared.screen().store;
        let id = match ids.id {
            0 => store.newest(ids.number),
            id => Some(id),
        };
        let image = id.and_then(|id| store.get(id)).ok_or(Error::NoImage)?;
        Ok((image.id, image.generation))
    }

    /// `a=f`'s first chunk: its image, then its data.
    pub(super) fn frame(&mut self, command: &Command, ids: Ids) -> Option<Reply> {
        let edit = command.int(b'r');
        let failed = |error| Some(Reply::failed(ids, error).frame(edit));
        if ids.id == 0 && ids.number == 0 {
            return failed(Error::NoId);
        }
        let (image, generation) = match self.named(ids) {
            Ok(named) => named,
            Err(error) => return failed(error),
        };
        let mut loading = match Loading::start(command, ids, self.pid) {
            Ok(loading) => loading,
            Err(error) => return failed(error),
        };
        loading.ids.id = image;
        loading.frame = Some(Ask {
            image,
            generation,
            at: (command.int(b'x'), command.int(b'y')),
            base: command.int(b'c'),
            edit,
            gap: command.int(b'z') as i32,
            overwrite: command.int(b'X') == 1,
            background: command.int(b'Y'),
        });
        if command.more() {
            self.loading = Some(loading);
            return None;
        }
        self.framed(loading)
    }

    /// A whole frame, composed onto its image's animation.
    pub(super) fn framed(&mut self, loading: Loading) -> Option<Reply> {
        let (pid, ids) = (self.pid, loading.ids);
        let ask = loading.frame.expect("a frame's load asks for a frame");
        let failed = |error| Some(Reply::failed(ids, error).frame(ask.edit));
        let data = match loading.finish(pid) {
            Ok(data) => data,
            Err(error) => return failed(error),
        };
        let mut shared = self.lock();
        let screen = shared.screen();
        let placed: Vec<u32> = screen.placements.iter().map(|kept| kept.image).collect();
        let store = &mut screen.store;
        // Deleted or sent again meanwhile, it is gone.
        let Some(image) = store
            .get(ask.image)
            .filter(|image| image.generation == ask.generation)
        else {
            return failed(Error::NoImage);
        };
        let size = (image.width, image.height);
        if data.width > size.0 || data.height > size.1 {
            return failed(Error::FrameTooLarge);
        }
        let count = image
            .animation
            .as_ref()
            .map_or(1, |animation| animation.count());
        let number = match ask.edit as usize {
            edit if edit == 0 || edit > count => count + 1,
            edit => edit,
        };
        let (from, at) = ((data.width, data.height), ask.at);
        if number > count {
            let base = match ask.base as usize {
                0 => None,
                base => match image.frame(base) {
                    Some(pixels) => Some(pixels.to_vec()),
                    None => return failed(Error::NoBase),
                },
            };
            let length = image.pixels.len();
            let reserved = store.reserve(ask.image, length, pid, &|id| placed.contains(&id));
            // Evicted images take their placements, though it failed.
            screen.orphans();
            if reserved.is_err() {
                return failed(Error::FrameSpace);
            }
            let mut canvas = base.unwrap_or_else(|| {
                let mut canvas = vec![0; length];
                fill(&mut canvas, ask.background);
                canvas
            });
            compose(&mut canvas, size, &data.pixels, from, at, ask.overwrite);
            let gap = match ask.gap {
                0 => GAP,
                gap => gap.max(0) as u32,
            };
            let image = screen.store.get_mut(ask.image);
            let image = image.expect("a reserve keeps its image");
            let animation = image.animation.get_or_insert_default();
            animation.frames.push(Frame {
                pixels: canvas,
                gap,
            });
        } else {
            let image = store.get_mut(ask.image).expect("the image was there");
            let animation = image.animation.get_or_insert_default();
            if ask.gap != 0 {
                animation.set_gap(number - 1, ask.gap.max(0) as u32);
            }
            let shown = animation.current + 1 == number;
            if shown {
                animation.shown_at = None;
            }
            let canvas = image.frame_mut(number).expect("an edited frame is there");
            compose(canvas, size, &data.pixels, from, at, ask.overwrite);
            if shown {
                image.changed();
            }
        }
        log::debug!(
            "graphics {pid}: image {} frame {number} composed",
            ask.image
        );
        let named = Ids {
            id: ask.image,
            ..ids
        };
        Some(Reply::ok(named).frame(number as u32))
    }

    /// `a=a`: gap, frame, state and loops; success answers nothing.
    pub(super) fn control(&mut self, command: &Command, ids: Ids) -> Option<Reply> {
        if ids.id == 0 && ids.number == 0 {
            return Some(Reply::failed(ids, Error::NoId));
        }
        let id = match self.named(ids) {
            Ok((id, _)) => id,
            Err(error) => return Some(Reply::failed(ids, error)),
        };
        let mut shared = self.lock();
        let image = shared.screen().store.get_mut(id).expect("named");
        let animation = image.animation.get_or_insert_default();
        let count = animation.count();
        let (frame, gap) = (command.int(b'r') as usize, command.int(b'z') as i32);
        if (1..=count).contains(&frame) && gap != 0 {
            animation.set_gap(frame - 1, gap.max(0) as u32);
        }
        let current = command.int(b'c') as usize;
        let moved = (1..=count).contains(&current) && current - 1 != animation.current;
        if moved {
            (animation.current, animation.shown_at) = (current - 1, None);
        }
        let state = match command.int(b's') {
            1 => Some(State::Stopped),
            2 => Some(State::Loading),
            3 => Some(State::Running),
            _ => None,
        };
        if let Some(state) = state {
            if animation.state == State::Stopped && state != State::Stopped {
                animation.shown_at = None;
            }
            (animation.state, animation.looped) = (state, 0);
            log::debug!("graphics {}: image {id} animation {state:?}", self.pid);
        }
        if command.int(b'v') != 0 {
            animation.loops = command.int(b'v') - 1;
        }
        if moved {
            image.changed();
        }
        None
    }

    /// `a=c`: frame `r`'s rectangle composed onto frame `c`'s.
    pub(super) fn compose(&mut self, command: &Command, ids: Ids) -> Option<Reply> {
        if ids.id == 0 && ids.number == 0 {
            return Some(Reply::failed(ids, Error::NoId));
        }
        let id = match self.named(ids) {
            Ok((id, _)) => id,
            Err(error) => return Some(Reply::failed(ids, error)),
        };
        let named = Ids { id, ..ids };
        let failed = |error| Some(Reply::failed(named, error));
        let mut shared = self.lock();
        let image = shared.screen().store.get_mut(id).expect("named");
        let (source, target) = (command.int(b'r') as usize, command.int(b'c') as usize);
        let Some(from) = image.frame(source) else {
            return failed(Error::NoSource);
        };
        if image.frame(target).is_none() {
            return failed(Error::NoDestination);
        }
        let (width, height) = (image.width, image.height);
        let wide = Some(command.int(b'w'))
            .filter(|&wide| wide > 0)
            .unwrap_or(width);
        let tall = Some(command.int(b'h'))
            .filter(|&tall| tall > 0)
            .unwrap_or(height);
        let (to, at) = (
            (command.int(b'x'), command.int(b'y')),
            (command.int(b'X'), command.int(b'Y')),
        );
        let fits = |(x, y): (u32, u32)| {
            u64::from(x) + u64::from(wide) <= u64::from(width)
                && u64::from(y) + u64::from(tall) <= u64::from(height)
        };
        if !fits(to) {
            return failed(Error::DestinationOut);
        }
        if !fits(at) {
            return failed(Error::SourceOut);
        }
        let overlaps = |a: u32, b: u32, length: u32| a.max(b) < a.min(b) + length;
        if source == target && overlaps(at.0, to.0, wide) && overlaps(at.1, to.1, tall) {
            return failed(Error::Overlap);
        }
        let row = |y: u32| {
            let start = ((at.1 + y) as usize * width as usize + at.0 as usize) * 4;
            &from[start..start + wide as usize * 4]
        };
        let rect: Vec<u8> = (0..tall).flat_map(row).copied().collect();
        let canvas = image.frame_mut(target).expect("checked");
        compose(
            canvas,
            (width, height),
            &rect,
            (wide, tall),
            to,
            command.int(b'C') == 1,
        );
        let current = image
            .animation
            .as_ref()
            .map_or(0, |animation| animation.current);
        if current + 1 == target {
            image.changed();
        }
        Some(Reply::ok(named))
    }

    /// `d=f`: frame `r` goes; `F` takes a still image.
    pub(super) fn delete_frame(&mut self, command: &Command, free: bool) {
        let pid = self.pid;
        let ids = Ids {
            id: command.int(b'i'),
            number: command.int(b'I'),
            placement: 0,
        };
        let Some((id, _)) = (ids.id > 0 || ids.number > 0)
            .then(|| self.named(ids).ok())
            .flatten()
        else {
            log::warn!("graphics {pid}: a frame delete names no image there");
            return;
        };
        let mut shared = self.lock();
        let screen = shared.screen();
        let image = screen.store.get(id).expect("named");
        if image
            .animation
            .as_ref()
            .is_none_or(|it| it.frames.is_empty())
        {
            if free && screen.remove(id) {
                log::debug!("graphics {pid}: image {id} deleted with no frames");
            }
            return;
        }
        let image = screen.store.get_mut(id).expect("named");
        let animation = image.animation.as_mut().expect("it has frames");
        // 0 is the first frame; past the last, the last.
        let number = (command.int(b'r') as usize).clamp(1, animation.count());
        let freed = match number {
            1 => {
                let promoted = animation.frames.remove(0);
                animation.root_gap = promoted.gap;
                std::mem::replace(&mut image.pixels, promoted.pixels).len()
            }
            number => animation.frames.remove(number - 2).pixels.len(),
        };
        let animation = image.animation.as_mut().expect("it had frames");
        let (gone, last) = (number - 1, animation.frames.len());
        let shown = animation.current > last || animation.current == gone;
        if shown {
            (animation.current, animation.shown_at) = (animation.current.min(last), None);
        } else if gone < animation.current {
            animation.current -= 1;
        }
        if shown {
            image.changed();
        }
        screen.store.release(freed);
        log::debug!("graphics {pid}: image {id} frame {number} deleted");
    }
}

#[cfg(test)]
mod tests;
