#![allow(unused)]
use anyhow::Result;
use dirs::home_dir;
use ignore::{Walk, WalkBuilder};
use serde::Deserialize;
use std::{
    fs::{read_dir, DirEntry},
    io::{stdout, Write},
    path::Path,
    process::Command,
    str::from_utf8,
    sync::mpsc,
};

fn main() -> Result<()> {
    let lr = get_local_repos_with_review_requests()?;
    println!("find local repos with review requests: {:?}", lr);
    create_worktrees(lr)?;

    Ok(())
}

#[derive(Debug, Clone)]
struct ReviewRequest {
    pub repo: GitRepo,
    pub id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitRepo {
    pub owner: String,
    pub repo: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GHRepository {
    pub name: String,
    pub name_with_owner: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GHReviewRequest {
    pub number: u64,
    pub repository: GHRepository,
}

impl From<String> for GitRepo {
    fn from(value: String) -> Self {
        let sv: Vec<&str> = value.split("/").collect();
        //todo: properly handle this out of bounds error
        let owner = sv.get(0).unwrap().to_string();
        let repo = sv.get(1).unwrap().to_string();

        GitRepo { owner, repo }
    }
}

impl From<GHReviewRequest> for ReviewRequest {
    fn from(value: GHReviewRequest) -> Self {
        ReviewRequest {
            repo: value.repository.name_with_owner.into(),
            id: value.number,
        }
    }
}

impl From<&GHReviewRequest> for ReviewRequest {
    fn from(value: &GHReviewRequest) -> Self {
        ReviewRequest {
            repo: value.repository.name_with_owner.clone().into(),
            id: value.number,
        }
    }
}

fn get_review_requests() -> Result<Vec<ReviewRequest>> {
    let o = Command::new("sh")
        .arg("-c")
        .arg("gh search prs --state=open --review-requested=@me --json \"repository,number\"")
        .output()?;
    let output_string = from_utf8(&o.stdout)?;

    let ghreq: Vec<GHReviewRequest> = serde_json::from_str(output_string)?;
    Ok(ghreq.iter().map(ReviewRequest::from).collect())
}

fn create_worktrees(reqs: Vec<LocalReviewRequest>) -> Result<()> {
    for req in reqs {
        let o = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "cd {} && git worktree add ./reviews/{}",
                req.path, req.branch
            ))
            .output()?;
        println!("Created worktree for {} at branch {}", req.repo.repo, req.branch);
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct LocalReviewRequest {
    pub branch: String,
    pub path: String,
    pub repo: GitRepo,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GHBranch {
    head_ref_name: String,
}

fn get_branch_name(pr_number: u64, path: String) -> Result<String> {
    let o = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cd {} && gh pr view {} --json \"headRefName\"",
            path, pr_number
        ))
        .output()?;
    let branch: GHBranch = serde_json::from_str(from_utf8(&o.stdout)?)?;

    Ok(branch.head_ref_name)
}

fn get_local_repos_with_review_requests() -> Result<Vec<LocalReviewRequest>> {
    let reqs = get_review_requests()?;
    let repos = find_repos();

    Ok(reqs
        .iter()
        .map(|req| {
            repos.iter().find(|v| v.repo == req.repo).and_then(|lr| {
                get_branch_name(req.id, lr.path.clone())
                    .map(|branch| {
                        Some(LocalReviewRequest {
                            branch,
                            path: lr.path.clone(),
                            repo: lr.repo.clone(),
                        })
                    })
                    .unwrap_or(None)
            })
        })
        .filter(|v| v.is_some())
        .map(|v| v.unwrap())
        .collect())
}

fn is_github_repo(path: &Path) -> bool {
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cd {} && gh repo view",
            path.to_str().unwrap_or("~")
        ))
        .output();

    if let Ok(o) = output {
        return o.status.code().map(|c| c == 0).unwrap_or(false);
    };

    false
}

#[derive(Debug, Clone)]
struct LocalRepo {
    pub path: String,
    pub repo: GitRepo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GHRepoViewJSON {
    name_with_owner: String,
}

fn build_local_repo(path: &Path) -> Result<LocalRepo> {
    let path_str = path.to_str().unwrap_or("~");
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cd {} && gh repo view --json \"nameWithOwner\"",
            path_str
        ))
        .output()?;

    let repo: GHRepoViewJSON = serde_json::from_str(from_utf8(&output.stdout)?)?;

    Ok(LocalRepo {
        path: path_str.to_string(),
        repo: GitRepo::from(repo.name_with_owner),
    })
}

fn find_repos() -> Vec<LocalRepo> {
    let mut repos = vec![];
    let (tx, mut rx) = mpsc::channel();

    let walker = WalkBuilder::new(&home_dir().unwrap())
        .standard_filters(true)
        .follow_links(false)
        .git_global(true)
        .same_file_system(true)
        .threads(6)
        .build_parallel();

    walker.run(|| {
        let tx = tx.clone();
        Box::new(move |result| {
            use ignore::WalkState::*;

            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if is_github_repo(path) {
                        match build_local_repo(path) {
                            Ok(repo) => {
                                tx.send(repo);
                            }
                            Err(err) => println!("ERROR: {}", err),
                        };
                        Skip
                    } else {
                        Continue
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
    while let Ok(res) = rx.recv() {
        repos.push(res);
    }

    repos
}
