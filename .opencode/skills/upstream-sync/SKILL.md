---
name: upstream-sync
description: Use when syncing changes from the upstream kanidm/kanidm repository into this kubidm fork. Use ONLY when the user asks to sync, rebase, merge, or bring changes from upstream/kanidm. Covers fetching upstream, identifying new commits, merging/rebasing, and resolving rename conflicts (kanidm -> kubidm).
---

# Upstream Sync: kanidm -> kubidm

This skill handles syncing changes from the upstream `kanidm/kanidm` repository into the `pando85/kubidm` (kubidm) fork.

## Repository Context

- **origin**: `git@github.com:pando85/kubidm.git` (this fork)
- **upstream**: `git@github.com:kanidm/kanidm.git` (original)
- **Main branch**: `master`
- **Fork-specific branch**: `master-fork` (contains fork-only history)

## Key Renames (kanidm -> kubidm)

The fork has renamed branding from `kanidm` to `kubidm` across many files. When syncing upstream changes, conflicts in these renamed files are expected. Key rename patterns:

- `kanidm` -> `kubidm` in crate names, binary names, documentation
- `kubidmd` -> `kubidmd` in daemon/service files
- `pykanidm` -> `pykubidm` in Python SDK
- Container images: `kanidm/server` -> `kubidm/server`, etc.
- Configuration files, service files, and docs reference `kubidm` branding

### Namespaces that must NOT be renamed

These are external crate names or upstream directory names that stay as `kanidm`:
- `kanidm-hsm-crypto` - external Rust crate, do NOT rename to `kanidm-hsm-crypto`
- Directory names under `unix_integration/` that upstream created: `resolver_kanidm/`, `nss_kanidm/`, `pam_kanidm/`, `rlm_kanidm/`
- `platform/` directory files
- `examples/` directory files (some)
- Debian packaging: `server/daemon/debian/kubidmd.service`

### Internal crate references that MUST be renamed

These workspace dependency keys and crate names were renamed by the fork:
- `kubidm_client` -> `kubidm_client` (both workspace key AND package name)
- `kubidm_proto` -> `kubidm_proto`
- `kubidm_core` -> `kubidm_core`
- `kubidmd_core` -> `kubidmd_core`
- `kubidmd_lib` -> `kubidmd_lib`
- `kubidmd_lib_macros` -> `kubidmd_lib_macros`
- `kubidmd_testkit` -> `kubidmd_testkit`
- `kubidm_build_profiles` -> `kubidm_build_profiles`
- `kubidm_lib_crypto` -> `kubidm_lib_crypto`
- `kubidm_lib_file_permissions` -> `kubidm_lib_file_permissions`
- `kubidm_utils_users` -> `kubidm_utils_users`
- `KubidmClient` -> `KubidmClient` (Rust struct name)
- `KubidmClientBuilder` -> `KubidmClientBuilder` (Rust struct name)

## Fork-Specific Code Additions

The fork has significant code additions beyond branding renames:
- **Approval workflows**: constants like `UUID_SCHEMA_ATTR_APPROVAL_*`, `UUID_SCHEMA_CLASS_APPROVAL_*`
- **Time-bounded grants**: `UUID_SCHEMA_ATTR_MEMBER_VALID_FROM`, `UUID_SCHEMA_ATTR_MEMBER_VALID_UNTIL`, `UUID_SCHEMA_ATTR_MAX_GRANT_DURATION`, `UUID_SCHEMA_CLASS_TIME_BOUNDED_GRANT`
- **OAuth2 enhancements**: `UUID_SCHEMA_ATTR_OAUTH2_ISSUER`, `UUID_SCHEMA_ATTR_OAUTH2_JWKS_URI`, etc.
- **AWS S3 integration**: `aws-config`, `aws-credential-types`, `aws-sdk-s3` dependencies

When accepting upstream files, these fork additions get lost. You MUST check for and re-add them.

## Sync Workflow

### Step 1: Assess Current State

```bash
git fetch upstream
git log --oneline $(git merge-base HEAD upstream/master)..upstream/master | wc -l
git log --oneline $(git merge-base HEAD upstream/master)..upstream/master
```

### Step 2: Plan the Sync Strategy

Before merging, review the new commits to identify:
- **High-risk changes**: large refactorings, file renames, Cargo.toml changes, build system changes
- **Security fixes**: prioritize these
- **Dependency updates**: usually safe to merge
- **Breaking changes**: API changes, schema changes, config format changes
- **Directory restructuring**: upstream may restructure directories (e.g., `resolver/` -> `resolver_common/` + `resolver_kanidm/`)

Present a summary to the user with:
1. Number of new commits
2. Key categories (security, features, deps, refactoring)
3. Recommended approach (merge vs rebase)
4. Risk assessment

### Step 3: Create a Sync Branch

```bash
git checkout master
git checkout -b sync/upstream-YYYY-MM-DD
```

### Step 4: Perform the Merge

```bash
git merge upstream/master --no-commit
```

### Step 5: Resolve Conflicts

#### Strategy for large merges (50+ conflicting files)

1. **Accept upstream for entire directories** that were restructured by upstream (e.g., `unix_integration/`). Then re-apply branding with sed.

2. **For code files**: Accept upstream version, then re-apply kubidm branding:
   ```bash
   git checkout --theirs <file>
   # Apply branding
   sed -i 's/kubidmd/kubidmd/g' <file>
   sed -i 's/kanidm/kubidm/g' <file>
   # Then FIX: restore external crate names that must not be renamed
   sed -i 's/kanidm-hsm-crypto/kanidm-hsm-crypto/g' <file>
   sed -i 's/kanidm_hsm_crypto/kanidm_hsm_crypto/g' <file>
   ```

3. **For Cargo.toml**: Accept upstream's version as the base, then:
   - Rename internal workspace dependency keys (kanidm_* -> kubidm_*, kubidmd_* -> kubidmd_*)
   - Add back fork-specific workspace dependencies (aws-*, etc.)
   - Do NOT rename external crate names (kanidm-hsm-crypto, etc.)
   - Add `kubidm_unix_common` as an alias for `sparkle_unix_common` (fork's pam_kubidm needs it)

4. **For files with fork-specific code additions** (constants, types, enums):
   - DO NOT blindly accept upstream. Instead, manually merge.
   - Extract fork additions from `git show HEAD:<file>` and apply on top of upstream's version.
   - Key files to watch: `server/lib/src/constants/uuids.rs`, `proto/src/attribute.rs`, `proto/src/constants.rs`

5. **Handle directory restructuring carefully**:
   - Remove fork's orphaned directories (e.g., `nss_kubidm/` superseded by upstream's `nss_kanidm/`)
   - Restore upstream directories that were deleted during merge (e.g., `nss_kanidm/`, `pam_kanidm/`)
   - Rename binary source files in `resolver_kanidm/src/bin/` to match Cargo.toml expectations

6. **Check for leftover diff markers** after resolution:
   ```bash
   grep -rl "<<<<<<" --include="*.rs" .
   ```

#### Common conflict resolution patterns

1. **Branding conflicts**: Always prefer `kubidm` branding over `kanidm`.
2. **Cargo.toml conflicts**: Accept upstream's dependency versions, add fork-specific deps back.
3. **Lock file conflicts**: Accept upstream's Cargo.lock, regenerate after all Cargo.toml files are fixed.
4. **Documentation conflicts**: Keep kubidm branding but accept upstream content changes.
5. **Code conflicts**: Accept upstream code changes, then re-apply branding.
6. **Fork-specific additions**: Extract from `git show HEAD:<file>` and re-apply on top of upstream.

### Step 6: Re-apply Branding

After accepting upstream files, systematically re-apply branding:

```bash
# Fix all .rs files (be careful with external crate names!)
for f in $(grep -rl "kubidm_proto\|kubidm_client\|KubidmClient" --include="*.rs" . | grep -v target/); do
  sed -i 's/KubidmClient/KubidmClient/g' "$f"
  sed -i 's/kubidm_proto/kubidm_proto/g' "$f"
  sed -i 's/kubidm_client/kubidm_client/g' "$f"
  # Restore external crate names
  sed -i 's/kanidm_hsm_crypto/kanidm_hsm_crypto/g' "$f"
done

# Fix all Cargo.toml files
for f in $(find . -name "Cargo.toml" -not -path "./target/*" -not -path "./Cargo.lock"); do
  sed -i 's/^kubidmd_/kubidmd_/g' "$f"
  sed -i 's/^kubidm_build_profiles/kubidm_build_profiles/' "$f"
  sed -i 's/^kubidm_client/kubidm_client/' "$f"
  # ... etc for each internal dep key
done
```

### Step 7: Verify

```bash
cargo check --workspace
cargo test --workspace  # if time permits
rg "kanidm" --glob "*.toml" --glob "*.rs" -l | grep -v "Cargo.lock" | head -20
```

### Step 8: Commit and Push

```bash
git add -A
git commit -m "sync: merge upstream kanidm/kanidm master (YYYY-MM-DD)"
git push origin sync/upstream-YYYY-MM-DD
```

## Critical Lessons Learned

### 1. The fork has SUBSTANTIVE code additions, not just renames

The fork adds approval workflows, time-bounded grants, OAuth2 enhancements, S3 backup integration, and more. Key files with fork-specific code:

**Files that should generally be restored from fork** (fork has significant additions beyond upstream):
- `server/lib/src/constants/uuids.rs` - 49+ fork-specific UUID constants
- `server/lib/src/schema.rs` - SyntaxType::TimeBoundedMember, new methods
- `server/lib/src/server/access/mod.rs` - ReauthRequired, Delegated variants
- `server/lib/src/server/access/create.rs` - fork-specific access control
- `server/lib/src/migration_data/dl14/mod.rs` - fork schema migrations
- `server/core/src/https/v1.rs` - approval API routes
- `server/core/src/https/v1_oauth2_federation.rs` - OAuth2 federation routes
- `server/core/src/https/authorization.rs` - authorization middleware
- `server/core/src/actors/v1_write.rs` - approval handler methods
- `server/core/src/config.rs` - S3 backup config, ServerRole
- `server/core/src/interval.rs` - S3 backup scheduling
- `server/core/src/backup/` - entire S3 backup module (directory)
- `server/daemon/src/main.rs` - fork-specific CLI commands (DbCommands::Recover, PitrList, etc.)
- `server/daemon/src/opt.rs` - KubidmdOpt variants
- `tools/cli/src/opt/kubidm.rs` - ApprovalOpt, SystemOpt::Approval
- `proto/src/config.rs` - ServerRole enum (deleted by upstream)
- `proto/src/backup.rs` - S3Config, WalArchiveConfig, PitrManifest
- `proto/src/cli.rs` - role field in KubidmdCli

**Strategy**: For these files, restore fork's version (`git show HEAD:<file> > <file>`), then apply branding renames with sed.

### 2. Restore-fork-then-rebrand is the correct strategy

Instead of accepting upstream and trying to add back fork code, the correct strategy for files with significant fork additions is:
```bash
git show HEAD:<file> > <file>
sed -i 's/kubidmd/kubidmd/g' <file>
sed -i 's/kanidm/kubidm/g' <file>
# Then restore external crate names
sed -i 's/kanidm-hsm-crypto/kanidm-hsm-crypto/g' <file>
sed -i 's/kanidm_hsm_crypto/kanidm_hsm_crypto/g' <file>
```

Then selectively integrate upstream's new additions (new methods, new imports) on top.

### 3. sed kanidm->kubidm needs follow-up restores

After `sed 's/kanidm/kubidm/g'`, ALWAYS restore:
- `kanidm-hsm-crypto` / `kanidm_hsm_crypto` (external crate)
- `ProviderOrigin::Kanidm` (enum variant - stays Kanidm, not Kubidm)
- `KanidmProvider` where the struct is actually named KanidmProvider in code
- `kanidm_flavour` when the lib crate name in Cargo.toml is `kubidm_flavour`
- Environment variables like `KANIDM_*` may or may not need renaming (check context)

### 4. Two-phase approach for large merges

Phase 1: Accept upstream for most files, resolve structural conflicts (directory renames, etc.)
Phase 2: Restore fork-specific files from `git show HEAD:<file>`, then selectively integrate upstream's new code additions.

This is faster and more reliable than trying to manually merge each conflict.

### 5. Common post-merge fix patterns

After the merge, expect to fix these categories iteratively:
1. **Missing fork constants/types** - restore fork's version of the file
2. **Missing fork API routes** - restore fork's v1.rs, add route_setup calls
3. **Missing fork CLI commands** - restore fork's opt.rs files
4. **Struct field mismatches** - add new upstream fields to fork code with defaults
5. **Enum variant mismatches** - add new upstream variants to fork's enums
6. **Method signature changes** - update call sites for upstream's new parameters
7. **Module visibility** - add `pub use` re-exports for types used across crates
8. **Directory-as-module conflicts** - backup.rs vs backup/ directory, remove .rs file

### 6. Build verification loop

After each batch of fixes, run `cargo check --workspace` and categorize remaining errors. Fix by category (constants first, then types, then imports, then patterns). This converges quickly.

## Security Notes

- Always review security-related commits carefully (look for "Security", "CVE", "vulnerability" in commit messages)
- Security fixes should be prioritized and synced ASAP
- The upstream may have security fixes that need careful backporting

## Post-Sync Checklist

- [ ] All conflicts resolved (no diff markers remaining)
- [ ] `cargo check --workspace` passes
- [ ] Fork-specific code additions preserved (constants, types, enums)
- [ ] No unintended kanidm->kubidm or kubidm->kanidm regressions
- [ ] External crate names preserved (kanidm-hsm-crypto, etc.)
- [ ] Workspace dependency keys consistent between root and individual Cargo.toml files
- [ ] Documentation references are consistent
- [ ] Docker image references are correct
- [ ] No orphaned directories from fork's old structure
- [ ] Create PR for review
