use super::*;

#[cfg(test)]
pub(super) fn log_sync(root: &str, scope: &str, limit: u32, skip: u32) -> RepoLog {
    log_sync_pinned(root, scope, limit, skip, None)
}
pub(super) fn log_sync_pinned(
    root: &str,
    scope: &str,
    limit: u32,
    skip: u32,
    tips: Option<&[String]>,
) -> RepoLog {
    let (git, repo_root) = match open_repository(root) {
        Ok(repository) => repository,
        Err(error) => {
            return RepoLog {
                commits: Vec::new(),
                has_more: false,
                truncated: false,
                error: Some(error),
                ..Default::default()
            };
        }
    };
    if !scope.is_empty()
        && (!scope.starts_with("refs/") || scope.chars().any(|character| character.is_whitespace()))
    {
        return RepoLog {
            commits: Vec::new(),
            has_more: false,
            truncated: false,
            error: Some("The history scope must be a full Git ref.".into()),
            ..Default::default()
        };
    }
    let limit = limit.clamp(1, MAX_LOG_PAGE);
    let format_arg = "--format=%H%x00%h%x00%P%x00%an%x00%ae%x00%at%x00%x00%s";
    let mut args: Vec<String> = vec![
        "log".into(),
        "--no-color".into(),
        "--no-show-signature".into(),
        "--topo-order".into(),
        format!("--max-count={}", limit + 1),
        format!("--skip={skip}"),
        format_arg.into(),
    ];
    if tips.is_some() {
        args.push("--stdin".into());
    } else if scope.is_empty() {
        args.push("--all".into());
    } else {
        args.push(scope.to_string());
    }
    let arguments: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = if let Some(tips) = tips {
        if tips.is_empty() {
            return RepoLog::default();
        }
        run_capture_input(
            &git,
            Some(&repo_root),
            &arguments,
            LOG_TIMEOUT,
            MAX_DIFF_BYTES,
            format!("{}\n", tips.join("\n")).into_bytes(),
        )
    } else {
        run_capture(
            &git,
            Some(&repo_root),
            &arguments,
            LOG_TIMEOUT,
            MAX_DIFF_BYTES,
        )
    };
    let output = match output {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            let message = stderr_message(&output);
            let empty = message.contains("does not have any commits yet")
                || message.contains("bad revision");
            return RepoLog {
                commits: Vec::new(),
                has_more: false,
                truncated: false,
                error: (!empty).then_some(message),
                ..Default::default()
            };
        }
        Err(error) => {
            return RepoLog {
                commits: Vec::new(),
                has_more: false,
                truncated: false,
                error: Some(error),
                ..Default::default()
            };
        }
    };
    let complete = if output.truncated {
        let end = output
            .stdout
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |end| end + 1);
        &output.stdout[..end]
    } else {
        &output.stdout
    };
    let mut commits = parse_log(complete);
    if output.truncated && commits.is_empty() {
        return RepoLog {
            error: Some("A commit exceeds the history output limit.".into()),
            truncated: true,
            ..Default::default()
        };
    }
    let has_more = commits.len() > limit as usize;
    if has_more {
        commits.truncate(limit as usize);
    }
    RepoLog {
        commits,
        has_more: has_more || output.truncated,
        truncated: output.truncated,
        error: None,
        ..Default::default()
    }
}

pub(super) fn parse_log(bytes: &[u8]) -> Vec<RepoCommit> {
    let text = String::from_utf8_lossy(bytes);
    let mut commits = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\0').collect();
        if fields.len() < 8 {
            continue;
        }
        let oid = fields[0].trim();
        if oid.is_empty() {
            continue;
        }
        commits.push(RepoCommit {
            oid: oid.to_string(),
            short: fields[1].trim().to_string(),
            parents: fields[2].split_whitespace().map(str::to_string).collect(),
            author: fields[3].to_string(),
            email: fields[4].to_string(),
            timestamp: fields[5].trim().parse().unwrap_or(0),
            refs: parse_refs(fields[6]),
            ref_count: parse_refs(fields[6]).len() as u32,
            subject: fields[7].to_string(),
        });
    }
    commits
}

pub(super) fn parse_refs(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

pub(super) fn commit_sync(root: &str, oid: &str) -> RepoCommitDetail {
    let (git, repo_root) = match open_repository(root) {
        Ok(repository) => repository,
        Err(error) => {
            return RepoCommitDetail {
                commit: RepoCommit::empty(oid.to_string()),
                body: String::new(),
                files: Vec::new(),
                patch: String::new(),
                truncated: false,
                error: Some(error),
            };
        }
    };
    if oid.is_empty()
        || oid.len() > 64
        || !oid.chars().all(|character| character.is_ascii_hexdigit())
    {
        return RepoCommitDetail {
            commit: RepoCommit::empty(oid.to_string()),
            body: String::new(),
            files: Vec::new(),
            patch: String::new(),
            truncated: false,
            error: Some("Not a commit hash.".into()),
        };
    }
    let format_arg = "--format=%H%x00%h%x00%P%x00%an%x00%ae%x00%at%x00%x00%s%x00%b";
    let meta = match run_capture(
        &git,
        Some(&repo_root),
        &[
            "show",
            "-s",
            "--no-color",
            "--no-show-signature",
            format_arg,
            oid,
        ],
        STATUS_TIMEOUT,
        MAX_STDERR_BYTES,
    ) {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            return RepoCommitDetail {
                commit: RepoCommit::empty(oid.to_string()),
                body: String::new(),
                files: Vec::new(),
                patch: String::new(),
                truncated: false,
                error: Some(stderr_message(&output)),
            };
        }
        Err(error) => {
            return RepoCommitDetail {
                commit: RepoCommit::empty(oid.to_string()),
                body: String::new(),
                files: Vec::new(),
                patch: String::new(),
                truncated: false,
                error: Some(error),
            };
        }
    };
    let meta_text = String::from_utf8_lossy(&meta.stdout);
    let fields: Vec<&str> = meta_text.split('\0').collect();
    let commit = if fields.len() >= 9 {
        RepoCommit {
            oid: fields[0].trim().to_string(),
            short: fields[1].trim().to_string(),
            parents: fields[2].split_whitespace().map(str::to_string).collect(),
            author: fields[3].to_string(),
            email: fields[4].to_string(),
            timestamp: fields[5].trim().parse().unwrap_or(0),
            refs: parse_refs(fields[6]),
            ref_count: parse_refs(fields[6]).len() as u32,
            subject: fields[7].to_string(),
        }
    } else {
        RepoCommit::empty(oid.to_string())
    };
    let body = fields
        .get(8)
        .map(|value| value.trim_end_matches(['\n', '\r']).to_string())
        .unwrap_or_default();
    let files = run_capture(
        &git,
        Some(&repo_root),
        &[
            "show",
            "--no-color",
            "--format=",
            "--name-status",
            "--find-renames",
            "--first-parent",
            oid,
        ],
        STATUS_TIMEOUT,
        MAX_STATUS_BYTES,
    )
    .map(|output| {
        if output.status.success() {
            parse_name_status(&output.stdout)
        } else {
            Vec::new()
        }
    })
    .unwrap_or_default();
    let patch = match run_capture(
        &git,
        Some(&repo_root),
        &[
            "show",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--format=",
            "--unified=3",
            "--find-renames",
            "--first-parent",
            oid,
        ],
        DIFF_TIMEOUT,
        MAX_DIFF_BYTES,
    ) {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            return RepoCommitDetail {
                commit,
                body,
                files,
                patch: String::new(),
                truncated: false,
                error: Some(stderr_message(&output)),
            };
        }
        Err(error) => {
            return RepoCommitDetail {
                commit,
                body,
                files,
                patch: String::new(),
                truncated: false,
                error: Some(error),
            };
        }
    };
    RepoCommitDetail {
        commit,
        body,
        files,
        patch: String::from_utf8_lossy(&patch.stdout).into_owned(),
        truncated: patch.truncated,
        error: None,
    }
}

pub(super) fn parse_name_status(bytes: &[u8]) -> Vec<RepoCommitFile> {
    let text = String::from_utf8_lossy(bytes);
    let mut files = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 2 {
            continue;
        }
        let status = fields[0].chars().next().unwrap_or('?').to_string();
        if (status == "R" || status == "C") && fields.len() >= 3 {
            files.push(RepoCommitFile {
                path: fields[2].to_string(),
                old_path: Some(fields[1].to_string()),
                status,
            });
        } else {
            files.push(RepoCommitFile {
                path: fields[1].to_string(),
                old_path: None,
                status,
            });
        }
    }
    files
}
