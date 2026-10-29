//! Getting around: where am i, go into, go back, go home.

use std::env;
use std::path::PathBuf;

use super::Reply;
use super::errors::{ActionError, friendly};
use super::paths::{existing, short};

pub(super) fn where_am_i() -> Result<Reply, ActionError> {
    Ok(Reply::Show(format!(
        "You are in {}",
        current_folder()?.display()
    )))
}

pub(super) fn go_into(name: &str) -> Result<Reply, ActionError> {
    let path = existing(name)?;
    if !path.is_dir() {
        return Err(ActionError::NotAFolder(name.to_string()));
    }
    env::set_current_dir(path).map_err(|error| friendly(error, name))?;
    you_are_now_in()
}

pub(super) fn go_back() -> Result<Reply, ActionError> {
    let here = current_folder()?;
    let Some(parent) = here.parent() else {
        return Ok(Reply::Show(
            "You're already at the very top folder.".to_string(),
        ));
    };
    env::set_current_dir(parent).map_err(|error| friendly(error, ".."))?;
    you_are_now_in()
}

pub(super) fn go_home() -> Result<Reply, ActionError> {
    let home = env::home_dir()
        .ok_or_else(|| ActionError::Other("I couldn't find your home folder.".to_string()))?;
    env::set_current_dir(&home).map_err(|error| friendly(error, "~"))?;
    you_are_now_in()
}

fn current_folder() -> Result<PathBuf, ActionError> {
    env::current_dir().map_err(|error| friendly(error, "."))
}

fn you_are_now_in() -> Result<Reply, ActionError> {
    Ok(Reply::Done(format!(
        "You are now in {}",
        short(&current_folder()?)
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::test_support::TestFolder;
    use std::fs;

    // Successful moves change the folder for the whole program, which would confuse
    // other tests running at the same time, so those are tested by running the real
    // program in `tests/cli.rs` instead.
    #[test]
    fn will_only_go_into_folders_that_exist() {
        let test = TestFolder::new("go-into");
        fs::write(test.at("a.txt"), "").unwrap();
        let before = env::current_dir().unwrap();

        assert!(matches!(
            go_into(&test.at("nope")),
            Err(ActionError::NotFound(_))
        ));
        assert!(matches!(
            go_into(&test.at("a.txt")),
            Err(ActionError::NotAFolder(_))
        ));
        assert_eq!(env::current_dir().unwrap(), before, "a failed go stays put");
    }
}
