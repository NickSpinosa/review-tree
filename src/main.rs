#![allow(unused)]
use dirs::home_dir;
use ignore::WalkBuilder;
use serde::Deserialize;
use std::{
    fmt::Display,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    str::from_utf8,
};
use thiserror::Error;

type AnyhowResult<T> = anyhow::Result<T>;

fn main() -> AnyhowResult<()> {
    let cfg = Config {
        num_threads: 16,
        root_dir: "/home/nick/code".into(),
        ..Config::default()
    };

    find_repos(cfg);

    Ok(())
}

struct Config {
    pub root_dir: String,
    pub num_threads: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root_dir: get_home_dir(),
            num_threads: 6,
        }
    }
}

fn get_home_dir() -> String {
    if let Some(path) = home_dir() {
        path.to_str().unwrap_or("/").to_owned()
    } else {
        "/".into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitRepo {
    pub owner: String,
    pub repo: String,
}

fn create_worktrees(reqs: Vec<LocalReviewRequest>) -> AnyhowResult<()> {
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

#[derive(Debug, Clone)]
struct LocalReviewRequest {
    pub branch: String,
    pub path: String,
    pub repo: GitRepo,
    pub title: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GHBranch {
    head_ref_name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GHPullRequest {
    head_ref_name: String,
    head_repository_owner: GHRepoOwner,
    head_repository: GHPRRepo,
    title: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GHPRRepo {
    id: String,
    name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GHRepoOwner {
    id: String,
    login: String,
}

impl From<GHPullRequest> for GitRepo {
    fn from(value: GHPullRequest) -> Self {
        GitRepo {
            owner: value.head_repository_owner.login,
            repo: value.head_repository.name,
        }
    }
}

impl From<&GHPullRequest> for GitRepo {
    fn from(value: &GHPullRequest) -> Self {
        GitRepo {
            owner: value.head_repository_owner.login.clone(),
            repo: value.head_repository.name.clone(),
        }
    }
}

impl GHPullRequest {
    fn to_local_review_request(self, path: &Path) -> LocalReviewRequest {
        LocalReviewRequest {
            branch: self.head_ref_name.clone(),
            path: path.to_str().unwrap_or("").to_owned(),
            title: self.title.clone(),
            repo: GitRepo::from(self),
        }
    }
}

#[derive(Error, Debug)]
enum BuildReviewRequestErrors {
    #[error("Not Git Repo Error")]
    NotGitRepoError,
    #[error("Not a GitHub Repo Error")]
    NotAGitHubRepoError,
    #[error("Local Git Repo Error")]
    LocalGitRepoError,
    #[error("UnknownGithubCliError {0}")]
    UnknownGithubCliError(String),
}

// todo: improve error handling
fn build_local_review_requests(path: &Path) -> AnyhowResult<Vec<LocalReviewRequest>> {
    let path_str = path.to_str().unwrap_or("~");
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cd {} && gh pr list -S \"user-review-requested:@me\" --json \"headRefName,headRepository,headRepositoryOwner,title\"",
            path_str
        ))
        .output()?;

    if !output.status.success() {
        let err = from_utf8(&output.stderr)?;
        use BuildReviewRequestErrors::*;

        return match err {
            "no git remotes found\n" => Err(LocalGitRepoError.into()),
            "failed to run git: fatal: not a git repository (or any of the parent directories): .git\n\n" => Err(NotGitRepoError.into()),
            "none of the git remotes configured for this repository point to a known GitHub host. To tell gh about a new GitHub host, please use `gh auth login`\n" => Err(NotAGitHubRepoError.into()),
            _ => {
                println!("path: {}", path_str);
                println!("gh error output: {:?}", err);
                Err(UnknownGithubCliError(err.into()).into())
            }
        };
    }

    let o = from_utf8(&output.stdout)?;
    let prs: Vec<GHPullRequest> = serde_json::from_str(o)?;

    Ok(prs
        .into_iter()
        .map(|pr| pr.to_local_review_request(path))
        .collect())
}

fn find_repos(cfg: Config) -> () {
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
