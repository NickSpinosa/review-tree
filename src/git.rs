use crate::prelude::*;
use serde::Deserialize;
use std::{path::Path, process::Command, str::from_utf8};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRepo {
    pub owner: String,
    pub repo: String,
}

impl Default for GitRepo {
    fn default() -> Self {
        Self {
            owner: Default::default(),
            repo: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalReviewRequest {
    pub branch: String,
    pub path: String,
    pub repo: GitRepo,
    pub title: String,
}

impl Default for LocalReviewRequest {
    fn default() -> Self {
        Self {
            branch: Default::default(),
            path: Default::default(),
            repo: Default::default(),
            title: Default::default(),
        }
    }
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
pub enum BuildReviewRequestErrors {
    #[error("Not Git Repo Error")]
    NotGitRepoError,
    #[error("Not a GitHub Repo Error")]
    NotAGitHubRepoError,
    #[error("Local Git Repo Error")]
    LocalGitRepoError,
    #[error("UnknownGithubCliError {0}")]
    UnknownGithubCliError(String),
}

pub fn build_local_review_requests(path: &Path) -> AnyhowResult<Vec<LocalReviewRequest>> {
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
