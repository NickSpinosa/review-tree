use crate::{core::ReviewRequestOutput, git::LocalReviewRequest, AnyhowResult};
use colored::Colorize;
use std::{io, process::Command, str::from_utf8};

pub fn create_worktree(req: &LocalReviewRequest) -> AnyhowResult<()> {
    let o = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cd {} && git worktree prune && git worktree add ./reviews/{} -b {}",
            req.path, req.branch, req.branch
        ))
        .output()?;

    Ok(())
}

pub fn create_tmux_session(req: &LocalReviewRequest) -> AnyhowResult<()> {
    let o = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cd {} && tmux new -d -s Code_Review:_{}",
            req.path,
            req.title.replace(" ", "_")
        ))
        .output()?;

    Ok(())
}

pub fn report_results(outputs: Vec<ReviewRequestOutput>) {
    let _ = io::stdout().lock();
    for output in outputs {
        println!(
            "{} {} {} {}",
            "Review request found for PR:".green(),
            output.review_request.title.blue().bold(),
            "to".green(),
            output.review_request.repo.repo.blue().bold()
        );

        if output.worktree_created {
            println!("{}  {}", ">".green(), "Worktree created!".green());
        }
        if output.tmux_session_created {
            println!("{}  {}", ">".green(), "Tmux session created!".green());
        }
    }
}
