#!/bin/sh
# Upload a release file and its stable alias.
set -eu
cd "$(dirname "$0")/.."
only_alias=false
if [ "${1:-}" = "--alias-only" ]; then
  only_alias=true
  shift
fi
[ "$#" -eq 1 ] || { echo "release-asset: use FILE or --alias-only FILE" >&2; exit 1; }
file=$1
[ -f "$file" ] || { echo "release-asset: no file $file" >&2; exit 1; }
version=$(cargo pkgid)
version=${version##*[#@]}
name=${file##*/}
case $name in
  ELYTerminal-[0-9]*.dmg) alias=ELYTerminal-macos-arm64.dmg ;;
  ELYTerminal-[0-9]*-windows-x86_64.zip) alias=ELYTerminal-windows-x86_64.zip ;;
  ELYTerminal-[0-9]*-linux-x86_64.AppImage) alias=ELYTerminal-linux-x86_64.AppImage ;;
  elyterminal_[0-9]*_amd64.deb) alias=elyterminal_amd64.deb ;;
  elyterminal-[0-9]*-1.x86_64.rpm) alias=elyterminal.x86_64.rpm ;;
  ELYTerminal-[0-9]*-linux-x86_64.tar.gz) alias=ELYTerminal-linux-x86_64.tar.gz ;;
  *) alias= ;;
esac
if [ "$only_alias" = true ] && [ -z "$alias" ]; then
  echo "release-asset: no stable alias for $name" >&2
  exit 1
fi
if [ "$only_alias" = false ]; then
  gh release upload "v$version" "$file" --clobber -R ELY-Open-Source/ELYTerminal
  echo "release-asset: $name on release v$version"
fi
if [ -n "$alias" ]; then
  work=$(mktemp -d)
  trap 'rm -rf "$work"' EXIT
  cp "$file" "$work/$alias"
  gh release upload "v$version" "$work/$alias" --clobber -R ELY-Open-Source/ELYTerminal
  echo "release-asset: $alias on release v$version"
fi
