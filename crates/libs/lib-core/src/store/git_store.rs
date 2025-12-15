use git2::{Repository, Signature, Oid, Error as GitError, Tree};
use std::path::Path;
use crate::Result;

/// Initializes a new Git repository at the given path.
/// If the repository already exists, it opens it.
pub fn init_or_open_repo<P: AsRef<Path>>(path: P) -> Result<Repository> {
    let path_ref = path.as_ref();
    if path_ref.exists() && Repository::discover(path_ref).is_ok() {
        Repository::open(path_ref).map_err(crate::Error::from)
    } else {
        Repository::init(path).map_err(crate::Error::from)
    }
}

/// Opens an existing Git repository at the given path.
pub fn open_repo<P: AsRef<Path>>(path: P) -> Result<Repository> {
    Repository::open(path).map_err(crate::Error::from)
}

/// Creates a commit with the given tree and signature
fn create_commit(
    repo: &Repository,
    tree: &Tree,
    signature: &Signature,
    message: &str,
) -> Result<Oid> {
    let parent_commit_opt = find_last_commit(repo)?;
    let commit_oid = match parent_commit_opt {
        Some(ref parent) => repo.commit(Some("HEAD"), signature, signature, message, tree, &[parent])?,
        None => repo.commit(Some("HEAD"), signature, signature, message, tree, &[])?,
    };
    Ok(commit_oid)
}

/// Commits a file (or changes to a file) to the repository.
pub fn commit_file<P: AsRef<Path>>(
    repo: &Repository,
    file_path: P,
    content: &str,
    message: &str,
    author_name: &str,
    author_email: &str,
) -> Result<Oid> {
    let workdir = repo.workdir().ok_or_else(|| GitError::from_str("No working directory"))?;
    let full_path = workdir.join(file_path.as_ref());

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full_path, content)?;

    let mut index = repo.index()?;
    index.add_path(file_path.as_ref())?;
    let tree = repo.find_tree(index.write_tree()?)?;
    let signature = Signature::now(author_name, author_email)?;

    create_commit(repo, &tree, &signature, message)
}

/// Commits multiple files (which must already exist on disk) to the repository.
pub fn commit_paths<P: AsRef<Path>>(
    repo: &Repository,
    paths: &[P],
    message: &str,
    author_name: &str,
    author_email: &str,
) -> Result<Oid> {
    let mut index = repo.index()?;
    for path in paths {
        index.add_path(path.as_ref())?;
    }
    let tree = repo.find_tree(index.write_tree()?)?;
    let signature = Signature::now(author_name, author_email)?;

    create_commit(repo, &tree, &signature, message)
}

/// Finds the last commit in the repository, returns None if no commits exist.
fn find_last_commit(repo: &Repository) -> Result<Option<git2::Commit<'_>>> {
    let head = repo.head();
    match head {
        Ok(head) => {
            let obj = head.resolve()?.peel(git2::ObjectType::Commit)?;
            let commit = obj.into_commit().map_err(|obj_not_commit| {
                GitError::from_str(&format!("Object is not a commit: {:?}", obj_not_commit.id()))
            })?;
            Ok(Some(commit))
        }
        Err(ref e) if e.code() == git2::ErrorCode::NotFound || e.code() == git2::ErrorCode::UnbornBranch => Ok(None), // Empty repo
        Err(e) => Err(crate::Error::from(e)),
    }
}

/// Reads the content of a file from the repository at HEAD.
pub fn read_file_content<P: AsRef<Path>>(repo: &Repository, file_path: P) -> Result<String> {
    let head = repo.head()?;
    let tree = head.peel_to_tree()?;
    let entry = tree.get_path(file_path.as_ref())?;
    let object = entry.to_object(repo)?;
    let blob = object.as_blob().ok_or_else(|| {
        GitError::from_str("Object is not a blob")
    })?;
    Ok(String::from_utf8_lossy(blob.content()).into_owned())
}