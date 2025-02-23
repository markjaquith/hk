use crate::Result;
use globset::{GlobBuilder, GlobSetBuilder};
use itertools::Itertools;
use std::path::{Path, PathBuf};

pub fn get_matches<P: AsRef<Path>>(glob: &[String], files: &[P]) -> Result<Vec<PathBuf>> {
    let files = files.iter().map(|f| f.as_ref()).collect_vec();
    let mut gb = GlobSetBuilder::new();
    for g in glob {
        let g = GlobBuilder::new(g).empty_alternates(true).build()?;
        gb.add(g);
    }
    let gs = gb.build()?;
    let matches = files
        .into_iter()
        .filter(|f| gs.is_match(f))
        .map(|f| f.to_path_buf())
        .collect_vec();
    Ok(matches)
}
