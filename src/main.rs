#![allow(unused)]
use anyhow::Result;
use dirs::home_dir;
use serde::Deserialize;
use std::{
    fs::{read_dir, DirEntry},
    io::{stdout, Write},
    path::Path,
    process::Command,
    str::from_utf8,
};

fn main() -> Result<()> {
    let requests = get_review_requests();
    println!("{:?}", requests);

    println!("is git repo ~: {}", is_get_repo(&Path::new("~")));
    println!("is git repo tftcalc: {}", is_get_repo(&Path::new("~/code/tftcalc")));
    println!("local repo tftcalc: {:?}", get_local_repo(&Path::new("~/code/tftcalc"))?);
    println!("find git repos: {:?}", find_repos());

    Ok(())
}

#[derive(Debug, Clone)]
struct ReviewRequest {
    pub repo: GitRepo,
    pub branch: String,
}

#[derive(Debug, Clone)]
struct GitRepo {
    pub owner: String,
    pub repo: String,
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

impl From<String> for ReviewRequest {
    fn from(value: String) -> Self {
        let rv: Vec<&str> = value.split("\t").collect();
        //todo: properly handle this out of bounds error
        let repo: GitRepo = rv.get(0).unwrap().to_string().into();
        let branch = rv.get(3).unwrap().to_string();

        ReviewRequest { repo, branch }
    }
}

fn get_review_requests() -> Result<Vec<ReviewRequest>> {
    let o = Command::new("sh")
        .arg("-c")
        .arg("gh search prs --state=open --review-requested=@me")
        .output()?;
    let output_string = from_utf8(&o.stdout)?;

    let requests = output_string
        .split("\n")
        .filter(|s| s != &"")
        .map(|r| ReviewRequest::from(r.to_owned()))
        .collect();

    Ok(requests)
}

fn is_get_repo(path: &Path) -> bool {
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("cd {} && git status", path.to_str().unwrap_or("~")))
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
    name_with_owner: String
}

fn get_local_repo(path: &Path) -> Result<LocalRepo> {
    let path_str = path.to_str().unwrap_or("~");
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("cd {} && gh repo view --json \"nameWithOwner\"", path_str))
        .output()?;

    let repo: GHRepoViewJSON = serde_json::from_str(from_utf8(&output.stdout)?)?;

    Ok(LocalRepo { path: path_str.to_string(), repo: GitRepo::from(repo.name_with_owner)})
}

fn find_repos() -> Vec<LocalRepo> {
    find_repos_rec(&home_dir().unwrap(), &mut Vec::new()).to_vec()
}

fn find_repos_rec<'r>(path: &Path, repos: &'r mut Vec<LocalRepo>) -> &'r mut Vec<LocalRepo> {
    println!("visiting path: {:?}", path);
    println!("path is dir: {}", path.is_dir());
    if path.is_dir() {
        if is_get_repo(path) {
            let lr = get_local_repo(path);
            // todo: don't swallow error
            if let Ok(repo) = lr {
                repos.push(repo);
            }
        }
        let dir = read_dir(path);

        if let Ok(subdirs) = dir {
            for entry in subdirs {
                let new_path = entry
                    .map(|e| e.path());

                if let Ok(np) = new_path {
                    &repos.append(find_repos_rec(&np, &mut repos.clone()));
                }
            }
        } 
    }
    
    repos
}
