pub mod local;
pub mod parser;
pub mod traits;

pub use local::LocalMediaStore;
pub use parser::DefaultMediaParser;
pub use traits::{
    FetchOptions, FetchedMedia, MediaEntry, MediaFetcher, MediaId, MediaMetadata, MediaParser,
    MediaStore, MediaToken,
};

use std::path::Path;

pub fn create_media_store(base_dir: &Path) -> Box<dyn MediaStore> {
    Box::new(LocalMediaStore::new(base_dir))
}

pub fn create_media_parser() -> Box<dyn MediaParser> {
    Box::new(DefaultMediaParser)
}
