# vcs-starship

Unified Starship custom module:
- Git repos: hardcoded `git_branch + git_state + git_status` output compatible with disabled built-in git modules.
- JJ and JJ+Git repos: existing jj-starship JJ output (`{symbol}{change_id} ({bookmarks}) [status]`).

## Build

```sh
cargo build --release
```

Binary path:

```sh
./target/release/vcs-starship
```

## Starship Configuration

Use the custom module and disable built-in git modules:

```toml
[custom.vcs]
when = "vcs-starship detect"
shell = ["vcs-starship"]
format = "$output "

[git_branch]
disabled = true

[git_status]
disabled = true

[git_state]
disabled = true
```

## Git Output Contract

Hardcoded style contract:
- Branch segment: `[branch(:remote_branch)]` (mauve)
- State segment: `[state( current/total)]` (yellow, progress only for rebase/am states)
- Status tokens:
  - staged `+N`
  - modified `!N`
  - conflicted `~N`
  - untracked `?N`
  - deleted `✘N`
  - ahead `⇡N`, behind `⇣N`, diverged `⇣behind⇡ahead`
  - stashed `*N`
  - renamed `ɍN`
  - optional line counts `+N -N` (`--lines tracked` or `--lines all`)

Hardcoded Catppuccin Macchiato color names: `mauve`, `yellow`, `peach`, `maroon`, `subtext0`, `red`, `green`.

## JJ Output

JJ output keeps jj-starship behavior:

```text
{symbol}{change_id} ({bookmarks}) [{status}]
```

`--lines tracked` appends `+N -N` for the snapshotted working-copy change.
`--lines all` also includes untracked files in Git. In JJ, both modes count
files in the working-copy snapshot; ignored or unsnapshotted files are excluded.
The default is `--lines none`, so no diff is calculated unless requested.

## Commands

```sh
vcs-starship prompt   # default command
vcs-starship detect   # exit 0 when inside a git/jj repo
vcs-starship version
```
