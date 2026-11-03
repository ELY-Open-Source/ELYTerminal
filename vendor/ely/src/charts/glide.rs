use std::time::Duration;

use web_time::Instant;

use crate::motion;

/// Values gliding from where they were to where the owner put them.
#[derive(Default)]
pub(crate) struct Glide {
    from: Vec<Vec<f64>>,
    to: Vec<Vec<f64>>,
    since: Option<Instant>,
}

impl Glide {
    /// The values at this moment, and whether they still move: a change glides from wherever the last glide stood; a new shape snaps.
    pub(crate) fn follow(
        &mut self,
        values: &[Vec<f64>],
        length: Duration,
    ) -> (Vec<Vec<f64>>, bool) {
        let share = |since: Option<Instant>| {
            since.map_or(1.0, |since| {
                (since.elapsed().as_secs_f32() / length.as_secs_f32()).min(1.0)
            })
        };
        if self.to != values {
            let now = if same_shape(&self.from, &self.to) {
                mix(
                    &self.from,
                    &self.to,
                    motion::ease_out_cubic(share(self.since)),
                )
            } else {
                self.to.clone()
            };
            let glides = !self.to.is_empty() && same_shape(&now, values);
            self.from = if glides { now } else { values.to_vec() };
            self.to = values.to_vec();
            self.since = glides.then(Instant::now);
        }
        let t = share(self.since);
        if t < 1.0 {
            (mix(&self.from, &self.to, motion::ease_out_cubic(t)), true)
        } else {
            (self.to.clone(), false)
        }
    }
}

/// Values `share` of the way from `from` to `to`.
fn mix(from: &[Vec<f64>], to: &[Vec<f64>], share: f32) -> Vec<Vec<f64>> {
    let share = f64::from(share);
    to.iter()
        .zip(from)
        .map(|(to, from)| {
            to.iter()
                .zip(from)
                .map(|(b, a)| a + (b - a) * share)
                .collect()
        })
        .collect()
}

fn same_shape(a: &[Vec<f64>], b: &[Vec<f64>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.len() == b.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glide_starts_still_then_moves_toward_a_change() {
        let (from, to) = (vec![vec![0.0, 10.0]], vec![vec![10.0, 20.0]]);
        assert_eq!(mix(&from, &to, 0.5), [vec![5.0, 15.0]]);
        let mut glide = Glide::default();
        let long = Duration::from_secs(60);
        assert_eq!(
            glide.follow(&from, long),
            (from.clone(), false),
            "the first values hold still"
        );
        let (now, moving) = glide.follow(&to, long);
        assert!(
            moving && now[0][0] < 1.0,
            "a change starts from the old values, got {now:?}"
        );
        assert_eq!(
            glide.follow(&[vec![1.0]], long),
            (vec![vec![1.0]], false),
            "a new shape snaps"
        );
    }
}
