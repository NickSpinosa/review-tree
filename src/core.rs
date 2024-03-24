use crate::{
    config::Args,
    git::{build_local_review_requests, BuildReviewRequestErrors, LocalReviewRequest},
    utils::{create_tmux_session, create_worktree },
};
use ignore::WalkBuilder;
use std::sync::mpsc;

pub struct ReviewRequestOutput {
    pub review_request: LocalReviewRequest,
    pub worktree_created: bool,
    pub tmux_session_created: bool,
    // pub error: Vec<Box<dyn Error>>,
}

impl Default for ReviewRequestOutput {
    fn default() -> Self {
        Self {
            review_request: Default::default(),
            worktree_created: Default::default(),
            tmux_session_created: Default::default(),
            // error: Default::default(),
        }
    }
}

pub fn find_review_requests(cfg: Args) -> Vec<ReviewRequestOutput> {
    let mut output = vec![];
    let (tx, rx) = mpsc::channel();

    let walker = WalkBuilder::new(cfg.root_dir)
        .standard_filters(true)
        .follow_links(false)
        .git_global(true)
        .same_file_system(true)
        .threads(cfg.num_threads)
        .build_parallel();

    walker.run(|| {
        let tx = tx.clone();
        Box::new(move |result| {
            use ignore::WalkState::*;

            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_dir() && !path.is_symlink() {
                        match build_local_review_requests(path) {
                            Ok(reqs) => {
                                for rr in reqs {
                                    let mut output = ReviewRequestOutput::default();

                                    if cfg.create_tmux_session {
                                        output.tmux_session_created = true;
                                        if let Err(err) = create_tmux_session(&rr) {
                                            println!("Error creating tmux session: {}", err);
                                            output.tmux_session_created = false;
                                        }
                                    }
                                    if cfg.create_worktree {
                                        output.worktree_created = true;
                                        if let Err(err) = create_worktree(&rr) {
                                            println!("Error creating worktree: {}", err);
                                            output.worktree_created = false;
                                        }
                                    }

                                    output.review_request = rr;
                                    let _ = tx.send(output);
                                }
                                Skip
                            }
                            Err(err) => {
                                use BuildReviewRequestErrors::*;

                                match err.downcast_ref::<BuildReviewRequestErrors>() {
                                    Some(NotAGitHubRepoError) => Skip,
                                    Some(LocalGitRepoError) => Skip,
                                    Some(NotGitRepoError) => Continue,
                                    Some(UnknownGithubCliError(msg)) => {
                                        // let _ =tx.send(output);
                                        Continue
                                    }
                                    None => Continue,
                                }
                            }
                        }
                    } else {
                        Skip
                    }
                }
                Err(err) => {
                    println!("WALK ERROR: {}", err);
                    Continue
                }
            }
        })
    });

    drop(tx);

    while let Ok(msg) = rx.recv() {
        output.push(msg);
    }

    output
}
