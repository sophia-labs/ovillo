#!/bin/sh
set -eu

revision="2d467bfec90af19301b01123494f4b2ba64c5a3a"
base_url="https://huggingface.co/datasets/Julian-br/GuitarJam/resolve/$revision"
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_dir=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
destination=${1:-"$repo_dir/artifacts/guitarjam/source"}

if ! command -v curl >/dev/null 2>&1; then
    echo "fetch-guitarjam: curl is required" >&2
    exit 1
fi

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d ' ' -f 1
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d ' ' -f 1
    else
        echo "fetch-guitarjam: sha256sum or shasum is required" >&2
        exit 1
    fi
}

fetch() {
    source_path=$1
    expected=$2
    output="$destination/$(basename -- "$source_path")"

    if [ -f "$output" ]; then
        actual=$(sha256 "$output")
        if [ "$actual" = "$expected" ]; then
            echo "verified $output"
            return
        fi
        echo "fetch-guitarjam: refusing to replace modified file $output" >&2
        exit 1
    fi

    partial="$output.part"
    trap 'rm -f -- "$partial"' EXIT HUP INT TERM
    curl --fail --location --proto '=https' --tlsv1.2 \
        "$base_url/$source_path?download=true" \
        --output "$partial"
    actual=$(sha256 "$partial")
    if [ "$actual" != "$expected" ]; then
        echo "fetch-guitarjam: digest mismatch for $source_path" >&2
        exit 1
    fi
    mv -- "$partial" "$output"
    trap - EXIT HUP INT TERM
    echo "fetched $output"
}

mkdir -p -- "$destination"
fetch "guitar_jam/test/026.wav" "700dbfa9399c944ba60bb03d2e598a704d9d28892393665c9433558cb3fe26ab"
fetch "guitar_jam/test/050.wav" "22e7b845fc2d71200c72c13fedbeb5f8ac593e48a01f99875f857e0cc5dfecd0"
fetch "guitar_jam/test/059.wav" "7f5e2989769aec91fddf217ee1c4ca90c186c2a1bd8a55d1781a2283bbc64248"
fetch "guitar_jam/test/061.wav" "350330311602f77500e8480db1f3875855205600cfe5f4893de74bea14d4cc9e"
fetch "guitar_jam/test/075.wav" "396c6a2f3925c96605fb059752b749a7dbe49066cb4f12dfda00e49b9b968a5d"

echo "GuitarJam CC0 fixture subset is ready in $destination"
