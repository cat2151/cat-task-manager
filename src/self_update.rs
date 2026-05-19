use std::error::Error;

const BUILD_COMMIT_HASH: &str = env!("BUILD_COMMIT_HASH");
const REPO_OWNER: &str = "cat2151";
const REPO_NAME: &str = "cat-task-manager";
const MAIN_BRANCH: &str = "main";

pub fn run_update() -> Result<(), Box<dyn Error>> {
    cat_self_update_lib::self_update(REPO_OWNER, REPO_NAME, &[])?;
    Ok(())
}

pub fn run_check() -> Result<(), Box<dyn Error>> {
    let result = cat_self_update_lib::check_remote_commit(
        REPO_OWNER,
        REPO_NAME,
        MAIN_BRANCH,
        BUILD_COMMIT_HASH,
    )?;
    println!("{result}");
    Ok(())
}
