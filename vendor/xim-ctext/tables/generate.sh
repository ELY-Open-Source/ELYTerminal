#!/bin/sh
# generate.sh: writes the tables from libX11 1.8.12's lcUniConv.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
for name in iso8859_1 iso8859_2 iso8859_3 iso8859_4 iso8859_5 iso8859_6 iso8859_7 iso8859_8 iso8859_9 \
  iso8859_10 iso8859_11 iso8859_13 iso8859_14 iso8859_15 iso8859_16 gb2312 jisx0208 jisx0212 ksc5601 big5 big5hkscs gbk; do
  curl -fsSL -o "$work/$name.h" \
    "https://gitlab.freedesktop.org/xorg/lib/libx11/-/raw/libX11-1.8.12/src/xlibi18n/lcUniConv/$name.h"
done
cc -std=c99 -Wall -Wno-unused-function -Werror -I "$work" -o "$work/generate" "$here/generate.c"
cd "$here"
"$work/generate"
