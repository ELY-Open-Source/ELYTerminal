use std::rc::Rc;

use gpui::{App, Window};

mod disasm;
mod flame;
mod hex;
mod network;
mod session;
mod stack;
#[cfg(all(test, feature = "test-support"))]
mod tests;
mod timeline;
mod variables;

pub use disasm::{Disassembly, Instruction};
pub use flame::{Flamegraph, Placed, ProfileFrame, place};
pub use hex::HexViewer;
pub use network::{NetworkInspector, Request};
pub use session::{Breakpoint, BreakpointList, DebugCommand, DebugState, DebugToolbar};
pub use stack::{CallStack, StackFrame, Thread, ThreadList, ThreadState};
pub(crate) use timeline::ruler;
pub use timeline::{Span, TimelineProfiler, zoomed};
pub use variables::{ValueKind, Variable, VariablesPanel, Watch, WatchPanel};

/// Tells the owner which row, by index.
type OnIndex = Rc<dyn Fn(usize, &mut Window, &mut App)>;
