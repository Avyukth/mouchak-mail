# Branch Protection Rules

## Current Setup (Local Hooks)

Since this is a private repo without GitHub Pro, we use local git hooks:

```bash
# Enable hooks for this repo
git config core.hooksPath .githooks
```

This prevents direct pushes to `main` branch.

---

## GitHub Branch Protection (When Available)

Apply these rules via **Settings → Branches → Add rule** for `main`:

### Required Settings

| Rule | Value | Why |
|------|-------|-----|
| **Require PR before merging** | ✅ | No direct commits to main |
| **Require approvals** | 1+ | Code review gate |
| **Dismiss stale reviews** | ✅ | Re-review after changes |
| **Require status checks** | ✅ | CI must pass |
| **Require branches up to date** | ✅ | No merge conflicts |
| **Require conversation resolution** | ✅ | Address all feedback |
| **Require signed commits** | Optional | Cryptographic verification |
| **Include administrators** | ✅ | Rules apply to everyone |
| **Restrict force pushes** | ✅ | Preserve history |
| **Restrict deletions** | ✅ | Prevent accidental deletion |

### Required Status Checks

Add these when CI is configured:

- `build` - Cargo build succeeds
- `test` - All tests pass
- `clippy` - No clippy warnings
- `fmt` - Code is formatted

### CLI Setup (GitHub Pro/Public)

```bash
# Enable branch protection via API
gh api repos/OWNER/REPO/branches/main/protection \
  -X PUT \
  -H "Accept: application/vnd.github+json" \
  -f required_status_checks='{"strict":true,"contexts":["build","test","clippy"]}' \
  -f enforce_admins=true \
  -f required_pull_request_reviews='{"required_approving_review_count":1,"dismiss_stale_reviews":true}' \
  -f restrictions=null \
  -f allow_force_pushes=false \
  -f allow_deletions=false
```

---

## Workflow Best Practices

1. **Always branch from main**:
   ```bash
   git checkout main && git pull
   git checkout -b feature/description
   ```

2. **Keep branches small**: Aim for <400 lines changed per PR

3. **Use conventional commits**:
   - `feat:` new feature
   - `fix:` bug fix
   - `refactor:` code change (no behavior change)
   - `docs:` documentation
   - `test:` tests
   - `chore:` maintenance

4. **PR checklist**:
   - [ ] Tests pass locally
   - [ ] Clippy has no warnings
   - [ ] Code is formatted (`cargo fmt`)
   - [ ] Commit messages are descriptive
