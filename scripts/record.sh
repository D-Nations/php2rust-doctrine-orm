#!/usr/bin/env bash
# Record test vectors from real PHP and the Doctrine packages (the oracle in
# oracle/, Docker) into crates/php2rust-doctrine-<name>/tests/data/.
#
#   scripts/record.sh TARGET           targets are added with their recorders
#
# Run scripts/fetch-upstream.sh first: the recorders load the packages and
# their tests from upstream/doctrine/<package>.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
export MSYS_NO_PATHCONV=1

if [[ ! -d upstream/doctrine/orm ]]; then
    echo "upstream/ is not fetched: run scripts/fetch-upstream.sh" >&2
    exit 1
fi

oracle() { docker compose -f oracle/compose.yaml run --rm -T php "$@"; }

# record RECORDER OUT: run RECORDER, check that it wrote JSON, move it to OUT.
record() {
    local recorder=$1 out=$2 tmp
    mkdir -p oracle/out "$(dirname "$out")"
    tmp=$(mktemp -p oracle/out)
    oracle "oracle/recorders/$recorder" > "$tmp"
    python -c 'import json,sys; json.load(open(sys.argv[1], encoding="utf-8"))' "$tmp"
    mv "$tmp" "$out"
    echo "written to $out"
}

case "${1:-}" in
    lexer) record lexer.php crates/php2rust-doctrine-lexer/tests/data/lexer.json ;;
    event-manager) record event_manager.php crates/php2rust-doctrine-event-manager/tests/data/event_manager.json ;;
    orm-query-lexer) record orm_query_lexer.php crates/php2rust-doctrine-orm/tests/data/query_lexer.json ;;
    inflector) record inflector.php crates/php2rust-doctrine-inflector/tests/data/inflector.json ;;
    *) echo "usage: scripts/record.sh lexer|event-manager|orm-query-lexer|inflector" >&2; exit 2 ;;
esac
