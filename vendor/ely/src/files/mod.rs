use std::rc::Rc;

use gpui::{App, Window};

use crate::lists::DirEntry;

mod archive;
mod columns;
mod duplicates;
mod explorer;
mod grid;
mod icon;
mod item;
mod operation;
mod preview;
mod recent;
mod search;
mod transfer;
mod transfers;

type OnEntry = Rc<dyn Fn(&DirEntry, &mut Window, &mut App)>;

#[cfg(all(test, feature = "test-support"))]
mod tests;

pub use archive::{ArchiveEntry, ArchiveViewer};
pub use duplicates::{DuplicateFinder, Duplicates};
pub use explorer::{FileExplorer, FileView};
pub use grid::FileGrid;
pub use icon::FileIcon;
pub use item::FileItem;
pub use operation::{FileOperation, FileOperationProgress};
pub use preview::FilePreview;
pub use recent::{RecentFile, RecentFiles};
pub use search::{FileSearch, FoundFile};
pub use transfer::{Transfer, TransferState};
pub use transfers::{DownloadManager, TransferQueue};
