#![allow(dead_code)]
use super::utils::repo;
use crate::error::Result;
use git2::StashFlags;
use scopetime::scope_time;

///
pub struct StashItems(Vec<String>);

///
pub fn get_stashes(repo_path: &str) -> Result<StashItems> {
    scope_time!("get_stashes");

    let mut repo = repo(repo_path)?;

    let mut list = Vec::new();

    repo.stash_foreach(|_index, msg, _id| {
        list.push(msg.to_string());
        true
    })?;

    Ok(StashItems(list))
}

///
fn stash(repo_path: &str, message: &str) -> Result<()> {
    scope_time!("stash");

    let mut repo = repo(repo_path)?;

    let sig = repo.signature()?;

    let options = StashFlags::INCLUDE_UNTRACKED;

    repo.stash_save(&sig, message, Some(options))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::tests::{get_statuses, repo_init};
    use std::{fs::File, io::Write};

    #[test]
    fn test_smoke() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert_eq!(stash(repo_path, "").is_ok(), false);

        assert_eq!(
            get_stashes(repo_path).unwrap().0.is_empty(),
            true
        );
    }

    #[test]
    fn test_stashing() -> Result<()> {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join("foo.txt"))?
            .write_all(b"test\nfoo")?;

        assert_eq!(get_statuses(repo_path), (1, 0));

        stash(repo_path, "stashname")?;

        assert_eq!(get_statuses(repo_path), (0, 0));

        Ok(())
    }

    #[test]
    fn test_stashes() -> Result<()> {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join("foo.txt"))?
            .write_all(b"test\nfoo")?;

        stash(repo_path, "foo")?;

        let res = get_stashes(repo_path)?;

        assert_eq!(res.0.len(), 1);
        assert_eq!(res.0[0], "On master: foo");

        Ok(())
    }
}
