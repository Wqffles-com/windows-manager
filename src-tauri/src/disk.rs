//! Folder-size scanner (WizTree-style): walks a drive in parallel and keeps the
//! tree in memory so the UI can drill down without rescanning.
//! shortcut: walks the directory tree instead of reading the NTFS MFT, so it is
//! slower than WizTree on huge drives and counts hard links once per link.

use rayon::prelude::*;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use tauri::State;

struct Node {
    name: Box<str>,
    size: u64,
    files: u64,
    is_dir: bool,
    children: Vec<Node>,
}

#[derive(Default)]
pub struct DiskState {
    tree: Mutex<Option<(PathBuf, Node)>>,
    scanned: AtomicU64,
    cancel: AtomicBool,
}

#[derive(Serialize)]
pub struct Entry {
    name: String,
    size: u64,
    files: u64,
    is_dir: bool,
}

#[derive(Serialize)]
pub struct Listing {
    size: u64,
    files: u64,
    entries: Vec<Entry>,
}

fn scan(path: &Path, name: Box<str>, st: &DiskState) -> Node {
    let mut children = Vec::new();
    let mut dirs = Vec::new();
    if !st.cancel.load(Relaxed) {
        // Unreadable folders (permissions, in use) are counted as empty.
        if let Ok(rd) = fs::read_dir(path) {
            for e in rd.flatten() {
                let Ok(ft) = e.file_type() else { continue };
                // Skips symlinks and junctions so nothing is counted twice or loops.
                if ft.is_symlink() {
                    continue;
                }
                let name: Box<str> = e.file_name().to_string_lossy().into();
                if ft.is_dir() {
                    dirs.push((e.path(), name));
                } else {
                    // On Windows this comes from the directory listing, no extra syscall.
                    let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                    children.push(Node { name, size, files: 1, is_dir: false, children: Vec::new() });
                }
            }
        }
    }
    st.scanned.fetch_add(children.len() as u64 + 1, Relaxed);
    children.extend(dirs.into_par_iter().map(|(p, n)| scan(&p, n, st)).collect::<Vec<_>>());
    children.sort_unstable_by_key(|c| std::cmp::Reverse(c.size));
    Node {
        name,
        size: children.iter().map(|c| c.size).sum(),
        files: children.iter().map(|c| c.files).sum(),
        is_dir: true,
        children,
    }
}

/// Finds a node by its name path from the scan root. Only paths that exist in
/// the scanned tree resolve, so the UI cannot point outside the scan.
fn find<'a>(mut node: &'a Node, path: &[String]) -> Option<&'a Node> {
    for part in path {
        node = node.children.iter().find(|c| *c.name == **part)?;
    }
    Some(node)
}

/// Removes the node at `path` and returns its (size, files) so ancestors can be updated.
fn remove(node: &mut Node, path: &[String]) -> Option<(u64, u64)> {
    let (first, rest) = path.split_first()?;
    let i = node.children.iter().position(|c| *c.name == **first)?;
    let freed = if rest.is_empty() {
        let c = node.children.remove(i);
        (c.size, c.files)
    } else {
        let freed = remove(&mut node.children[i], rest)?;
        node.children.sort_unstable_by_key(|c| std::cmp::Reverse(c.size));
        freed
    };
    node.size -= freed.0;
    node.files -= freed.1;
    Some(freed)
}

#[tauri::command]
pub async fn scan_start(path: String, state: State<'_, Arc<DiskState>>) -> Result<(), String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("{path} is not a folder"));
    }
    let st = Arc::clone(&state);
    st.cancel.store(false, Relaxed);
    st.scanned.store(0, Relaxed);
    *st.tree.lock().unwrap() = None;
    let tree = tauri::async_runtime::spawn_blocking({
        let st = Arc::clone(&st);
        let root = root.clone();
        move || scan(&root, path.into(), &st)
    })
    .await
    .map_err(|e| e.to_string())?;
    if st.cancel.load(Relaxed) {
        return Err("Scan cancelled".into());
    }
    *st.tree.lock().unwrap() = Some((root, tree));
    Ok(())
}

#[tauri::command]
pub fn scan_progress(state: State<'_, Arc<DiskState>>) -> u64 {
    state.scanned.load(Relaxed)
}

#[tauri::command]
pub fn scan_cancel(state: State<'_, Arc<DiskState>>) {
    state.cancel.store(true, Relaxed);
}

#[tauri::command]
pub fn dir_children(path: Vec<String>, limit: usize, state: State<'_, Arc<DiskState>>) -> Result<Listing, String> {
    let guard = state.tree.lock().unwrap();
    let (_, root) = guard.as_ref().ok_or("Nothing scanned yet")?;
    let node = find(root, &path).ok_or("Folder not found in scan")?;
    Ok(Listing {
        size: node.size,
        files: node.files,
        entries: node
            .children
            .iter()
            .take(limit)
            .map(|c| Entry { name: c.name.to_string(), size: c.size, files: c.files, is_dir: c.is_dir })
            .collect(),
    })
}

#[tauri::command]
pub fn entry_path(path: Vec<String>, state: State<'_, Arc<DiskState>>) -> Result<String, String> {
    let guard = state.tree.lock().unwrap();
    let (root_path, root) = guard.as_ref().ok_or("Nothing scanned yet")?;
    find(root, &path).ok_or("Not found in scan")?;
    Ok(path.iter().fold(root_path.clone(), |p, part| p.join(part)).to_string_lossy().into())
}

/// Moves a scanned file or folder to the Recycle Bin, never a permanent delete.
#[tauri::command]
pub async fn delete_entry(path: Vec<String>, state: State<'_, Arc<DiskState>>) -> Result<(), String> {
    if path.is_empty() {
        return Err("Refusing to delete the scan root".into());
    }
    let full = PathBuf::from(entry_path(path.clone(), state.clone())?);
    tauri::async_runtime::spawn_blocking(move || trash::delete(full))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    if let Some((_, root)) = state.tree.lock().unwrap().as_mut() {
        remove(root, &path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_find_remove() {
        let dir = std::env::temp_dir().join(format!("wm-disk-test-{}", std::process::id()));
        fs::create_dir_all(dir.join("a/b")).unwrap();
        fs::write(dir.join("a/b/big.bin"), vec![0u8; 3000]).unwrap();
        fs::write(dir.join("small.txt"), b"hi").unwrap();

        let st = DiskState::default();
        let mut root = scan(&dir, "root".into(), &st);
        assert_eq!((root.size, root.files), (3002, 2));
        assert_eq!(&*root.children[0].name, "a"); // sorted by size, largest first

        let p = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(find(&root, &p(&["a", "b"])).unwrap().size, 3000);
        assert!(find(&root, &p(&["..", "x"])).is_none());

        assert_eq!(remove(&mut root, &p(&["a", "b", "big.bin"])), Some((3000, 1)));
        assert_eq!((root.size, root.files), (2, 1));
        assert_eq!(find(&root, &p(&["a"])).unwrap().size, 0);
        fs::remove_dir_all(dir).unwrap();
    }
}
