#!/bin/sh
set -eu

# Builds the temporary fixtures used by the screenshot acceptance tests.
# Everything lives under /tmp; no user files, Git repositories or shell
# startup files are read.

demo=/tmp/termlevel-screenshot-demo
home=/tmp/termlevel-screenshot-home
shell=/tmp/termlevel-screenshot-shell

mkdir -p "$demo" "$home"

cat > "$demo/README.md" <<'EOF'
# Termlevel

A native macOS terminal workspace built around vertical tabs,
tab groups and split panes.

## Why

Keep shells, dev servers, tests and coding-agent sessions
in one window without losing track of the keyboard.

## Highlights

- Firefox-style vertical tabs and tab groups
- Real PTY sessions with the Ghostty terminal core
- Split panes with stable session IDs
- Git Changes and History for every group
- A Files tree rooted at the group directory
- Light, dark and system themes

## Try

  make test
  make run
  make package

The terminal keeps running when a pane is hidden, collapsed,
renamed or remounted. Only an explicit close ends a session.
EOF

{
  echo "# Release notes"
  echo
  echo "This file is demo data for a screenshot."
  echo
  i=1
  while [ "$i" -le 60 ]; do
    echo "## 0.$i.0"
    echo
    echo "Improved split-pane focus, Git refresh and file watching."
    echo
    i=$((i + 1))
  done
} > "$demo/NOTES.md"

rm -rf "$demo/demo-repo"
git init -q -b main "$demo/demo-repo"
cd "$demo/demo-repo"
git config user.name "Termlevel Demo"
git config user.email "demo@example.com"
printf '# Demo repository\n' > README.md
git add README.md
git commit -q -m "Initial workspace"
printf 'export type Layout = { kind: "pane" | "split" };\n' > tiling.ts
git add tiling.ts
git commit -q -m "Keep pane geometry stable"
printf 'export function Sidebar() { return null; }\n' > sidebar.tsx
git add sidebar.tsx
git commit -q -m "Group tabs by workspace"
git branch feature/tiling
git tag v0.1
git checkout -q feature/tiling
printf 'export type Layout = { kind: "pane" | "split"; ratio: number };\n' > tiling.ts
git add tiling.ts
git commit -q -m "Tune split ratios"
git checkout -q main

cat > "$shell" <<'EOF'
#!/bin/sh
unset ENV BASH_ENV PROMPT_COMMAND
export HOME=/tmp/termlevel-screenshot-home
export BASH_SILENCE_DEPRECATION_WARNING=1
export PS1='❯ '
exec /bin/bash --noprofile --norc -i
EOF
chmod +x "$shell"

echo "Screenshot fixtures are ready in $demo"
