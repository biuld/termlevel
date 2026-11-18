#!/bin/sh
set -eu
project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
kind=${1:-}
case "$kind" in
  preview) fixture=Preview; report=screenshots-preview ;;
  status) fixture=Status; report=screenshots-status ;;
  clis) fixture=Clis; report=screenshots-clis ;;
  *) echo 'Usage: tests/screenshots/run.sh {preview|status|clis} [NSH test options]' >&2; exit 2 ;;
esac
shift
if [ "$kind" = preview ]; then
  export TERMLEVEL_PREVIEW=1
fi
# NSH resolves open() relative to the test module. The acceptance sources use
# root-relative fixture paths, so copy them to a temporary root runner as
# scripts/test-native does.
runner_base=$(mktemp "$project_dir/.screenshot-$kind.XXXXXX")
runner="$runner_base.test.tsx"
trap 'rm -f "$runner" "$runner_base"' EXIT HUP INT TERM
cp "$project_dir/tests/screenshots/$fixture.acceptance.tsx" "$runner"
"$project_dir/scripts/nsh" test "$runner" --target macos --junit "artifacts/$report.xml" "$@"
