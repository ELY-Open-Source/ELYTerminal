//! The website's glyph swarm: springs, dust, the pointer and shocks.

use std::cell::UnsafeCell;
use std::f32::consts::TAU;

const MAX_COUNT: u32 = 1 << 20;
const SUBSTEP: f32 = 1.0 / 120.0;
const MAX_DT: f32 = 0.25;
const MAX_SPEED: f32 = 4000.0;
const WANDER: f32 = 16.0;
const PUSH: f32 = 160_000.0;
const CELL_OMEGA: f32 = 9.0;
const CELL_ZETA: f32 = 0.72;
const DUST_OMEGA: f32 = 1.6;
const DUST_ZETA: f32 = 1.0;

struct Frame {
    origin: [f32; 2],
    cell: [f32; 2],
    size: [f32; 2],
    time: f32,
}

impl Frame {
    fn new(origin: [f32; 2], cell: [f32; 2], size: [f32; 2], time: f32) -> Frame {
        let values = [
            origin[0], origin[1], cell[0], cell[1], size[0], size[1], time,
        ];
        assert!(
            values.iter().all(|value| value.is_finite()),
            "swarm: a frame value is not finite"
        );
        Frame {
            origin,
            cell,
            size,
            time,
        }
    }
}

struct Traits {
    stiffness: f32,
    frequency: [f32; 2],
    phase: [f32; 2],
    heading: [f32; 2],
}

#[derive(Clone, Copy)]
enum Kind {
    Dust,
    Cell,
    Pixel,
}

impl Kind {
    fn of(home: [f32; 3]) -> Kind {
        match home[2] {
            0.0 => Kind::Dust,
            1.0 => Kind::Cell,
            2.0 => Kind::Pixel,
            _ => panic!("swarm: a particle's kind is not 0, 1 or 2"),
        }
    }

    fn target(self, home: [f32; 3], frame: &Frame, traits: &Traits) -> [f32; 2] {
        let [x, y, _] = home;
        match self {
            Kind::Dust => [0, 1].map(|axis| {
                let wave = frame.time * traits.frequency[axis] + traits.phase[axis];
                [x, y][axis] * frame.size[axis] + WANDER * wave.sin()
            }),
            Kind::Cell => [
                frame.origin[0] + x * frame.cell[0],
                frame.origin[1] + y * frame.cell[1],
            ],
            Kind::Pixel => [x, y],
        }
    }

    fn spring(self, traits: &Traits) -> (f32, f32) {
        let (omega, zeta) = match self {
            Kind::Dust => (DUST_OMEGA, DUST_ZETA),
            Kind::Cell | Kind::Pixel => (CELL_OMEGA, CELL_ZETA),
        };
        let stiffness = omega * omega * traits.stiffness;
        (stiffness, 2.0 * zeta * stiffness.sqrt())
    }
}

struct Random(u32);

impl Random {
    fn new(seed: u32) -> Random {
        let mut random = Random((seed ^ 0x9E37_79B9).max(1));
        for _ in 0..8 {
            random.next();
        }
        random
    }

    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1 << 24) as f32
    }
}

struct Swarm {
    homes: Box<[[f32; 3]]>,
    output: Box<[[f32; 3]]>,
    velocity: Box<[[f32; 2]]>,
    traits: Box<[Traits]>,
}

impl Swarm {
    fn new(count: usize, seed: u32) -> Swarm {
        let mut random = Random::new(seed);
        let traits = (0..count)
            .map(|_| {
                let heading = random.next() * TAU;
                Traits {
                    stiffness: 0.6 + 0.8 * random.next(),
                    frequency: [0.25 + 0.35 * random.next(), 0.25 + 0.35 * random.next()],
                    phase: [random.next() * TAU, random.next() * TAU],
                    heading: [heading.cos(), heading.sin()],
                }
            })
            .collect();
        Swarm {
            homes: vec![[0.0; 3]; count].into_boxed_slice(),
            output: vec![[0.0; 3]; count].into_boxed_slice(),
            velocity: vec![[0.0; 2]; count].into_boxed_slice(),
            traits,
        }
    }

    fn step(&mut self, dt: f32, frame: &Frame, pointer: [f32; 3]) {
        assert!(
            (0.0..=MAX_DT).contains(&dt),
            "swarm: dt is outside 0 to 0.25 s"
        );
        assert!(
            pointer.iter().all(|value| value.is_finite()) && pointer[2] >= 0.0,
            "swarm: the pointer is not finite with a radius of 0 or more"
        );
        let substeps = (dt / SUBSTEP).ceil().max(1.0);
        let h = dt / substeps;
        for (index, out) in self.output.iter_mut().enumerate() {
            let home = self.homes[index];
            let traits = &self.traits[index];
            let kind = Kind::of(home);
            let target = kind.target(home, frame, traits);
            let (stiffness, damping) = kind.spring(traits);
            let mut position = [out[0], out[1]];
            let mut velocity = self.velocity[index];
            for _ in 0..substeps as u32 {
                let push = falloff(position, pointer, traits.heading);
                for axis in 0..2 {
                    let force = stiffness * (target[axis] - position[axis]) + PUSH * push[axis];
                    velocity[axis] = (velocity[axis] + force * h) / (1.0 + damping * h);
                }
                velocity = clamp(velocity);
                for axis in 0..2 {
                    position[axis] += velocity[axis] * h;
                }
            }
            self.velocity[index] = velocity;
            *out = [position[0], position[1], length(velocity)];
        }
    }

    fn shock(&mut self, center: [f32; 2], strength: f32, radius: f32) {
        assert!(
            strength.is_finite() && radius.is_finite() && radius > 0.0,
            "swarm: the shock is not finite with a positive radius"
        );
        for (index, out) in self.output.iter_mut().enumerate() {
            let push = falloff(
                [out[0], out[1]],
                [center[0], center[1], radius],
                self.traits[index].heading,
            );
            let velocity = &mut self.velocity[index];
            *velocity = clamp([0, 1].map(|axis| velocity[axis] + strength * push[axis]));
            out[2] = length(*velocity);
        }
    }

    fn snap(&mut self, frame: &Frame) {
        for (index, out) in self.output.iter_mut().enumerate() {
            let home = self.homes[index];
            let [x, y] = Kind::of(home).target(home, frame, &self.traits[index]);
            *out = [x, y, 0.0];
            self.velocity[index] = [0.0; 2];
        }
    }
}

fn falloff(position: [f32; 2], source: [f32; 3], heading: [f32; 2]) -> [f32; 2] {
    let [x, y, radius] = source;
    let offset = [position[0] - x, position[1] - y];
    let distance = length(offset);
    if distance >= radius {
        return [0.0; 2];
    }
    let weight = (1.0 - distance / radius).powi(2);
    // A particle on the center leaves along its own heading.
    let direction = if distance < 1e-3 {
        heading
    } else {
        offset.map(|value| value / distance)
    };
    direction.map(|value| value * weight)
}

fn clamp(velocity: [f32; 2]) -> [f32; 2] {
    let speed = length(velocity);
    if speed > MAX_SPEED {
        velocity.map(|value| value * MAX_SPEED / speed)
    } else {
        velocity
    }
}

fn length(vector: [f32; 2]) -> f32 {
    (vector[0] * vector[0] + vector[1] * vector[1]).sqrt()
}

struct Slot(UnsafeCell<Option<Swarm>>);

// SAFETY: wasm calls one export at a time.
unsafe impl Sync for Slot {}

static SWARM: Slot = Slot(UnsafeCell::new(None));

fn slot() -> &'static mut Option<Swarm> {
    // SAFETY: wasm calls one export at a time.
    unsafe { &mut *SWARM.0.get() }
}

fn swarm() -> &'static mut Swarm {
    slot().as_mut().expect("swarm: init was not called")
}

#[unsafe(no_mangle)]
pub extern "C" fn init(count: u32, seed: u32) {
    assert!(
        (1..=MAX_COUNT).contains(&count),
        "swarm: count is outside 1 to 2^20"
    );
    let slot = slot();
    assert!(slot.is_none(), "swarm: init was called twice");
    *slot = Some(Swarm::new(count as usize, seed));
}

#[unsafe(no_mangle)]
pub extern "C" fn homes() -> *mut f32 {
    swarm().homes.as_mut_ptr().cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn output() -> *mut f32 {
    swarm().output.as_mut_ptr().cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn step(
    dt: f32,
    time: f32,
    origin_x: f32,
    origin_y: f32,
    cell_w: f32,
    cell_h: f32,
    width: f32,
    height: f32,
    pointer_x: f32,
    pointer_y: f32,
    pointer_radius: f32,
) {
    let frame = Frame::new(
        [origin_x, origin_y],
        [cell_w, cell_h],
        [width, height],
        time,
    );
    swarm().step(dt, &frame, [pointer_x, pointer_y, pointer_radius]);
}

#[unsafe(no_mangle)]
pub extern "C" fn shock(x: f32, y: f32, strength: f32, radius: f32) {
    swarm().shock([x, y], strength, radius);
}

#[unsafe(no_mangle)]
pub extern "C" fn snap(
    origin_x: f32,
    origin_y: f32,
    cell_w: f32,
    cell_h: f32,
    width: f32,
    height: f32,
    time: f32,
) {
    let frame = Frame::new(
        [origin_x, origin_y],
        [cell_w, cell_h],
        [width, height],
        time,
    );
    swarm().snap(&frame);
}
