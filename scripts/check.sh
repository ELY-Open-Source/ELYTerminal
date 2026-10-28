#!/bin/sh
# Pre-review gate: build rules plus house rules.
set -eu
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --quiet
fail=0
files=$(git ls-files -co --exclude-standard '*.rs')
multi=$(awk 'FNR==1{prev=0} /^[[:space:]]*\/\// {if(prev) print FILENAME":"FNR; prev=1; next} {prev=0}' $files)
[ -z "$multi" ] || { echo "multi-line comments:"; echo "$multi"; fail=1; }
for f in $(git ls-files -co --exclude-standard); do
  case ${f##*/} in Cargo.lock | package-lock.json) continue ;; esac
  grep -Iq '' "$f" || continue
  n=$(wc -l < "$f")
  [ "$n" -le 500 ] || { echo "$f has $n lines"; fail=1; }
done
exit $fail
