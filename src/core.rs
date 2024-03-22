use ignore::WalkBuilder;

use crate::{config::Config, git::{build_local_review_requests, BuildReviewRequestErrors}, utils::create_worktrees};

pub fn find_repos(cfg: Config) -> () {
    let walker = WalkBuilder::new(cfg.root_dir)
        .standard_filters(true)
        .follow_links(false)
        .git_global(true)
        .same_file_system(true)
        .threads(cfg.num_threads)
        .build_parallel();

    walker.run(|| {
        Box::new(move |result| {
            use ignore::WalkState::*;

            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_dir() && !path.is_symlink() {
                        match build_local_review_requests(path) {
                            Ok(rr) => {
                                if let Err(err) = create_worktrees(rr) {
                                    println!("error creating worktree: {}", err);
                                }
                                Skip
                            }
                            Err(err) => {
                                use BuildReviewRequestErrors::*;
                                match err.downcast_ref::<BuildReviewRequestErrors>() {
                                    Some(NotAGitHubRepoError) => Skip,
                                    Some(LocalGitRepoError) => Skip,
                                    Some(NotGitRepoError) => Continue,
                                    Some(UnknownGithubCliError(msg)) => Continue,
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
}
