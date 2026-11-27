# TackleCast Fork Guide (`Srlimao/TackleCast`)

This repository is a personal fork of `SaltedByte/TackleCast`. Keep fork-only automation and notes isolated so upstream PRs stay clean.

## Fork Structure & Golden Rules

- **`fork/branch-release`** (GitHub default branch & integration branch for `Srlimao/TackleCast`): Holds fork-only files—[`.github/workflows/branch-release.yml`](.github/workflows/branch-release.yml) and `AGENTS.md`—and accumulates all active feature/fix branches into a single combined build for friends. GitHub requires `workflow_dispatch` workflows to live on the repository's default branch so the **Run workflow** button appears in **Actions**.
- **`main`**: Clean mirror of upstream `SaltedByte/TackleCast` `main`.
- **Feature / PR branches**: **Always cut and edit new work and upstream PR branches from `main`**, never commit feature code directly to `fork/branch-release`, and never merge `fork/branch-release` back into `main` or any feature/PR branch.
- **Never manually bump `Cargo.toml` for preview releases**: Leave `Cargo.toml` and `Cargo.lock` untouched on feature branches and on `fork/branch-release`. The `Branch Release (Fork)` workflow stamps the preview version in CI only during packaging.

---

## Workflow 1: Adding or Updating Features for Friends

1. **Create and commit your feature on a branch off `main`:**
   ```powershell
   git switch main
   git pull origin main
   git switch -c feat/my-change
   # make changes & commit
   ```
2. **Merge the feature branch into `fork/branch-release` and push:**
   ```powershell
   git switch fork/branch-release
   git merge feat/my-change
   git push origin fork/branch-release
   ```
3. **Build & publish the combined Windows preview on GitHub:**
   - Open **Srlimao/TackleCast → Actions → Branch Release (Fork) → Run workflow**.
   - Keep **Use workflow from** set to `fork/branch-release` and **`branch`** at its default (`fork/branch-release`).
   - Leave **`version`** blank to auto-pick the next unused `vMAJOR.MINOR.100+` tag (`v3.0.100`, `v3.0.101`, …), or specify a custom strict `vMAJOR.MINOR.PATCH`.
   - Optionally enable **`run_checks`** (`cargo fmt --all --check`, `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`) or add **`release_notes`**.
   - Click **Run workflow**.
4. **What friends do:**
   - **First install:** Download `TackleCast-Windows-x86_64.zip` from `https://github.com/Srlimao/TackleCast/releases/latest` (direct zip: `https://github.com/Srlimao/TackleCast/releases/latest/download/TackleCast-Windows-x86_64.zip`), extract it into a writable folder, and run `TackleCast.exe`.
   - **Subsequent updates:** Click **Check for updates** inside TackleCast. Builds compile with `TACKLECAST_UPDATE_REPO=Srlimao/TackleCast` and publish as non-prerelease `/releases/latest` releases, so the app detects newer versions automatically (1-click in-app install when Ed25519 keys are configured, or opens the GitHub release page otherwise).
5. **Opening an upstream PR to `SaltedByte/TackleCast`:**
   - Push your feature branch (`git push -u origin feat/my-change`) and open the PR from `Srlimao:feat/my-change` → `SaltedByte:main`.
   - Because `feat/my-change` branched off `main`, neither `branch-release.yml`, `AGENTS.md`, other merged branches, nor CI-stamped versions appear in the PR.

---

## Workflow 2: When Official (`SaltedByte/TackleCast`) Releases a New Version

1. **Sync your fork's `main` branch with upstream:**
   - Via GitHub UI: open `https://github.com/Srlimao/TackleCast/tree/main` and click **Sync fork → Update branch** (or pull from an `upstream` remote locally).
   - Pull locally:
     ```powershell
     git switch main
     git pull origin main
     ```
2. **Rebase any still-unmerged active feature branches onto the updated `main`:**
   ```powershell
   git switch feat/my-change
   git rebase main
   git push --force-with-lease origin feat/my-change
   ```
3. **Update `fork/branch-release` with `main` and active feature branches:**
   ```powershell
   git switch fork/branch-release
   git merge main
   # also merge any updated unmerged feature branches still needed
   git push origin fork/branch-release
   ```
   *(If prior rebases or dropped PRs ever make `fork/branch-release` messy, you can cleanly rebuild `fork/branch-release` from `main` + the fork-only commits for `.github/workflows/branch-release.yml` and `AGENTS.md` + fresh merges of your currently active branches.)*
4. **Publish a new fork build** via **Actions → Branch Release (Fork) → Run workflow**:
   - Keep **`branch`** as `fork/branch-release` and leave **`version`** blank.
   - Auto-versioning avoids upstream tag collisions while preserving upgrade ordering:
     - **Upstream minor/major release (e.g., `v3.1.0`):** `Cargo.toml` is `3.1.0`, so the workflow jumps to **`v3.1.100`** (`v3.1.101`, …). Friends on `v3.0.10x` auto-update to `3.1.100`.
     - **Upstream patch release (e.g., `v3.0.1`):** If the fork already published `v3.0.102`, the workflow picks **`v3.0.103`**—no conflict with `v3.0.1`, and friends on `v3.0.102` still see `3.0.103 > 3.0.102`.
5. **Maintaining `branch-release.yml`:**
   - Usually no changes are needed when upstream releases, because the workflow runs the checkout's own packaging/verification scripts.
   - Only update [`.github/workflows/branch-release.yml`](.github/workflows/branch-release.yml) if upstream changes the Rust toolchain version, cache keys, or packaging flags in [`.github/workflows/release.yml`](.github/workflows/release.yml).

---

## Ed25519 Auto-Updater Signing Setup (Optional, for 1-Click In-App Updates)

Without signing keys, `Branch Release (Fork)` still builds and detects newer fork versions, falling back to opening the GitHub release page in the browser. To enable 1-click in-app installation:

1. **Generate a fork Ed25519 keypair locally** (OpenSSL 3 + Python):
   ```powershell
   openssl genpkey -algorithm Ed25519 -out fork-update-key.pem
   python -c "import subprocess; der = subprocess.check_output(['openssl', 'pkey', '-in', 'fork-update-key.pem', '-pubout', '-outform', 'DER']); print('PUBLIC_KEY:', der[-32:].hex())"
   ```
2. **Configure GitHub Actions settings** in `Srlimao/TackleCast` → **Settings → Secrets and variables → Actions**:
   - **`TACKLECAST_UPDATE_PUBLIC_KEY`** (Variable or Secret): the 64-character hex `PUBLIC_KEY`.
   - **`TACKLECAST_UPDATE_SIGNING_KEY`** (Secret): the full PEM contents of `fork-update-key.pem`. Keep `fork-update-key.pem` backed up offline and never commit it to the repository.
