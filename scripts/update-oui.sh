#!/usr/bin/env bash
#
# Refresh the vendored IEEE OUI snapshot at data/oui.tsv.gz.
#
# Downloads the IEEE MA-L registry CSV, reduces it to compact "PREFIX\tVendor"
# rows, sorts them, and writes a gzip-compressed TSV. Run this when you want to
# update the embedded database; commit the resulting data/oui.tsv.gz.
#
# Usage: ./scripts/update-oui.sh
set -euo pipefail

OUI_URL="${OUI_URL:-https://standards-oui.ieee.org/oui/oui.csv}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$REPO_ROOT/data/oui.tsv.gz"
TMP_CSV="$(mktemp)"
trap 'rm -f "$TMP_CSV"' EXIT

echo "==> Downloading IEEE OUI registry from $OUI_URL"
curl -fSL --retry 3 --retry-delay 5 -o "$TMP_CSV" "$OUI_URL"

bytes=$(wc -c < "$TMP_CSV")
if [ "$bytes" -lt 1000000 ]; then
    echo "error: downloaded file is suspiciously small ($bytes bytes); aborting" >&2
    exit 1
fi

echo "==> Transforming to compact TSV"
python3 - "$TMP_CSV" "$OUT" <<'PY'
import csv, gzip, io, sys

src, out = sys.argv[1], sys.argv[2]
rows = []
with open(src, newline="", encoding="utf-8") as f:
    reader = csv.reader(f)
    next(reader, None)  # header
    for row in reader:
        if len(row) < 3:
            continue
        assign = row[1].strip().upper()
        org = " ".join(row[2].strip().split())
        if len(assign) != 6 or not org:
            continue
        rows.append((assign, org))

rows.sort(key=lambda r: r[0])

buf = io.StringIO()
for assign, org in rows:
    buf.write(f"{assign}\t{org}\n")

with gzip.open(out, "wb", compresslevel=9) as g:
    g.write(buf.getvalue().encode("utf-8"))

print(f"    wrote {len(rows)} entries to {out}")
PY

echo "==> Done. Review and commit data/oui.tsv.gz"
