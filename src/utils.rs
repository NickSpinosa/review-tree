use std::process::Command;

use crate::{git::LocalReviewRequest, AnyhowResult};


pub fn create_worktrees(reqs: Vec<LocalReviewRequest>) -> AnyhowResult<()> {
    for req in reqs {
        let o = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "cd {} && git worktree add ./reviews/{}",
                req.path, req.branch
            ))
            .output()?;
        println!(
            "Created worktree for {} at branch {}",
            req.repo.repo, req.branch
        );
    }

    Ok(())
}
