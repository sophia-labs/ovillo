#!/usr/bin/env bash
# release.sh (SKETCH, not run): build the graph + site on the laptop, copy a release to the box, install it, and
# (with --switch) swap `current` and load the graph. Mirrors deploy/two-box/cow-tools/stage-release.sh.
#
#   release.sh <name> <gardend linux x86_64 binary> <shrubbery app dist dir> [--switch]
#
# The release holds gardend and site/ (the app build + media/ scores/ receipts/ + llms.txt). The graph travels as
# ovillo.nt beside the release and is loaded into a fresh profile (`gardend load` refuses while gardend serves it),
# so a release = a profile rebuild: stop, load into profile.new, swap, start. Rollback = the previous pair.
set -euo pipefail
NAME=$1; GD=$2; APP=$3; SWITCH=${4:-}
HERE=$(cd "$(dirname "$0")/.." && pwd)            # site/graph
HOST=${SIRIN_INSTANCE_ID:?set SIRIN_INSTANCE_ID to the server's EC2 instance id (reached over SSM)}
: "${AWS_PROFILE:?set AWS_PROFILE}" "${AWS_REGION:?set AWS_REGION}"
SSH=(ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -o ConnectTimeout=25 -o "ProxyCommand=aws ssm start-session --target %h --document-name AWS-StartSSHSession --parameters portNumber=%p" -i "$HOME/.ssh/id_ed25519" "operator@$HOST")
file "$GD" | grep -q 'ELF 64-bit LSB pie executable, x86-64' || { echo "REFUSED: $GD is not a linux x86_64 executable" >&2; exit 1; }
[[ -f $APP/index.html ]] || { echo "REFUSED: $APP has no index.html" >&2; exit 1; }
W=$(mktemp -d); trap 'rm -rf "$W"' EXIT
uv run --with numpy --with soundfile --with rdflib python "$HERE/build_graph.py" --out "$W/out"
uv run --with numpy --with soundfile --with rdflib python "$HERE/test_graph.py"        # real data; refuses a release that fails
mkdir -p "$W/r"; cp "$GD" "$W/r/gardend"; chmod 0755 "$W/r/gardend"
cp -R "$APP" "$W/r/site"; cp -R "$W/out/site/." "$W/r/site/"   # media/ scores/ receipts/ over the app
cp "$W/out/ovillo.nt" "$W/r/ovillo.nt"
printf '{"name":"%s","gardendSha256":"%s","ntSha256":"%s","assembledAt":"%s"}\n' "$NAME" "$(shasum -a 256 "$GD" | cut -c1-64)" \
  "$(shasum -a 256 "$W/r/ovillo.nt" | cut -c1-64)" "$(date -u +%FT%TZ)" > "$W/r/RELEASE.json"
COPYFILE_DISABLE=1 tar -C "$W/r" --no-xattrs -czf "$W/$NAME.tar.gz" .
SUM=$(shasum -a 256 "$W/$NAME.tar.gz" | cut -c1-64)
echo "release $NAME: $(du -h "$W/$NAME.tar.gz" | cut -f1) sha256 $SUM"
"${SSH[@]}" "umask 077; cat > /tmp/ovillo-$NAME.tar.gz" < "$W/$NAME.tar.gz"
"${SSH[@]}" "sudo -n ovillo-release install '$NAME' /tmp/ovillo-$NAME.tar.gz $SUM && rm -f /tmp/ovillo-$NAME.tar.gz"   # ovillo-release: to write, as cow-tools-release
if [[ $SWITCH == --switch ]]; then "${SSH[@]}" "sudo -n ovillo-release switch '$NAME'"; fi
