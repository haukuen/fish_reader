use std::collections::HashMap;

use super::FileEntry;

pub(super) enum DiffAction {
    Upload(String),
    Delete(String),
    Download(String),
}

pub(super) fn diff_for_upload(
    local: &HashMap<String, FileEntry>,
    remote: &HashMap<String, FileEntry>,
) -> Vec<DiffAction> {
    let mut actions = Vec::new();

    for (path, local_entry) in local {
        match remote.get(path) {
            Some(remote_entry) if remote_entry.hash == local_entry.hash => {}
            _ => actions.push(DiffAction::Upload(path.clone())),
        }
    }

    for path in remote.keys() {
        if !local.contains_key(path) {
            actions.push(DiffAction::Delete(path.clone()));
        }
    }

    actions
}

pub(super) fn diff_for_download(
    local: &HashMap<String, FileEntry>,
    remote: &HashMap<String, FileEntry>,
) -> Vec<DiffAction> {
    let mut actions = Vec::new();

    for (path, remote_entry) in remote {
        match local.get(path) {
            Some(local_entry) if local_entry.hash == remote_entry.hash => {}
            _ => actions.push(DiffAction::Download(path.clone())),
        }
    }

    for path in local.keys() {
        if !remote.contains_key(path) {
            actions.push(DiffAction::Delete(path.clone()));
        }
    }

    actions
}
