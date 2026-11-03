use std::{
    any::TypeId,
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

use gpui::{App, Asset, ElementId, Global, RenderImage, Window};

type Key = (TypeId, u64);

/// Each held asset's holders and its painted image.
#[derive(Default)]
struct Holders(HashMap<Key, (usize, Option<Arc<RenderImage>>)>);

impl Global for Holders {}

impl Holders {
    fn join(&mut self, key: Key) {
        self.0.entry(key).or_default().0 += 1;
    }

    fn loaded(&mut self, key: Key, image: Option<Arc<RenderImage>>) {
        let record = self.0.get_mut(&key).expect("a loaded asset is held");
        if image.is_some() {
            record.1 = image;
        }
    }

    /// One holder leaves; the last gets the image to free.
    fn leave(&mut self, key: Key) -> Option<Option<Arc<RenderImage>>> {
        let record = self.0.get_mut(&key).expect("a held asset is counted");
        record.0 -= 1;
        if record.0 > 0 {
            return None;
        }
        self.0.remove(&key).map(|(_, image)| image)
    }
}

fn key<A: Asset>(source: &A::Source) -> Key {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    (TypeId::of::<A>(), hasher.finish())
}

/// An element's asset; the last holder to leave frees it.
pub(crate) struct Held<A: Asset> {
    source: Option<A::Source>,
}

impl<A: Asset> Held<A> {
    fn take(&mut self, source: &A::Source, cx: &mut App) {
        cx.default_global::<Holders>().join(key::<A>(source));
        self.source = Some(source.clone());
    }

    fn free(&mut self, cx: &mut App) {
        let Some(source) = self.source.take() else {
            return;
        };
        let Some(image) = cx.default_global::<Holders>().leave(key::<A>(&source)) else {
            return;
        };
        cx.remove_asset::<A>(&source);
        if let Some(image) = image {
            // The window may be drawing; free after it.
            cx.defer(move |cx| cx.drop_image(image, None));
        }
    }
}

/// `source` as `A` for element `id`; the previous released.
pub(crate) fn hold<A: Asset>(
    id: impl Into<ElementId>,
    source: &A::Source,
    image: impl Fn(&A::Output) -> Option<Arc<RenderImage>>,
    window: &mut Window,
    cx: &mut App,
) -> Option<A::Output>
where
    A::Source: PartialEq,
{
    let held = window.use_keyed_state(id, cx, |_, cx| {
        cx.on_release(|held: &mut Held<A>, cx| held.free(cx))
            .detach();
        Held { source: None }
    });
    let loaded = window.use_asset::<A>(source, cx);
    held.update(cx, |held, cx| {
        if held.source.as_ref() != Some(source) {
            held.free(cx);
            held.take(source, cx);
        }
    });
    if let Some(loaded) = &loaded {
        cx.default_global::<Holders>()
            .loaded(key::<A>(source), image(loaded));
    }
    loaded
}

#[cfg(test)]
mod tests {
    use gpui::RenderImage;
    use smallvec::smallvec;

    use super::*;

    fn picture() -> Arc<RenderImage> {
        let frame = image::Frame::new(image::RgbaImage::new(1, 1));
        Arc::new(RenderImage::new(smallvec![frame]))
    }

    #[test]
    fn the_last_holder_frees_the_image_another_painted() {
        let (mut holders, key) = (Holders::default(), (TypeId::of::<()>(), 7));
        holders.join(key);
        holders.join(key);
        let painted = picture();
        holders.loaded(key, Some(painted.clone()));
        // The pending holder reports nothing yet.
        holders.loaded(key, None);
        assert!(holders.leave(key).is_none(), "one holder remains");
        let freed = holders.leave(key).expect("the last holder left");
        assert_eq!(freed.map(|image| image.id), Some(painted.id));
        assert!(holders.0.is_empty());
    }
}
