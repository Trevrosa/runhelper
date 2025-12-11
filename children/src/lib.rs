//! mostly taken from [harryplusplus/kill_tree](https://github.com/harryplusplus/kill-tree/blob/main/crates/libs/kill_tree/)

// TODO: linux/mac?

#[cfg(windows)]
mod windows;

use std::collections::{HashMap, VecDeque};

#[cfg(windows)]
use windows as imp;

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub name: String,
    pub pid: u32,
    pub parent_pid: u32,
}
/// returns the children of the `parent`.
pub fn get_children<'a>(parent: u32) -> anyhow::Result<Vec<ProcessInfo>> {
    let processes = imp::get_processes()?;

    let mut process_map: HashMap<u32, Vec<ProcessInfo>> = HashMap::new();
    for process in processes {
        if imp::process_filter(&process) {
            continue;
        }

        let children = process_map.entry(process.parent_pid).or_default();
        children.push(process);
    }

    let mut children = Vec::new();

    // look at all the pids that are children of the parent, including children-of-children, etc.
    let mut nested_pids: VecDeque<u32> = VecDeque::new();
    nested_pids.push_back(parent);
    while let Some(pid) = nested_pids.pop_front() {
        if let Some(nested_children) = process_map.get(&pid) {
            for child in nested_children {
                if child.pid != parent {
                    children.push(child.clone());
                }
                nested_pids.push_back(child.pid);
            }
        }
    }

    Ok(children)
}
