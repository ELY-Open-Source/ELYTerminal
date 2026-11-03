#!/bin/sh
# Packs the Nerd Font faces the terminal registers into one xz archive.
#
# The release tarball carries 98 faces across six variants and unpacks to 243 MB;
# the terminal draws JetBrainsMonoNerdFontMono's eight weights and their italics,
# and xz packs those to about 2.5 MB since the faces share nearly every outline.
# Run this after bumping NF to a new release.
set -eu

NF=${NF:-v3.5.1}
URL="https://github.com/ryanoasis/nerd-fonts/releases/download/${NF}/JetBrainsMono.tar.xz"
WEIGHTS="Thin ExtraLight Light Regular Medium SemiBold Bold ExtraBold"

root=$(cd "$(dirname "$0")/.." && pwd)
out=$root/assets/fonts/JetBrainsMonoNerdFontMono.tar.xz
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

echo "fonts: fetching $URL"
curl -fsSL "$URL" -o "$work/release.tar.xz"
tar -xf "$work/release.tar.xz" -C "$work"

pick=$work/pick
mkdir "$pick"
for weight in $WEIGHTS; do
    for face in "JetBrainsMonoNerdFontMono-$weight.ttf" \
                "JetBrainsMonoNerdFontMono-${weight}Italic.ttf"; do
        file=$work/$face
        [ -f "$file" ] || { echo "fonts: no $face in $NF" >&2; exit 1; }
        cp "$file" "$pick/"
    done
done

tar -cf "$work/weights.tar" -C "$pick" .
xz -9e --check=crc64 -c "$work/weights.tar" > "$out"

raw=$(cat "$pick"/*.ttf | wc -c)
packed=$(wc -c < "$out")
echo "fonts: $raw bytes raw, $packed packed, written to $out"
