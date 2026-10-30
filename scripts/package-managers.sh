#!/bin/sh
# Write this version's Homebrew cask and winget manifests; --tap pushes the cask, --winget opens a PR.
set -eu
cd "$(dirname "$0")/.."
version=$(cargo pkgid)
version=${version##*[#@]}
repo=ELY-Open-Source/ELYTerminal
digest() {
  sum=$(gh release view "v$version" -R "$repo" --json assets \
    --jq ".assets[] | select(.name == \"$1\") | .digest" | sed 's/^sha256://')
  [ -n "$sum" ] || { echo "package-managers: release v$version holds no $1" >&2; exit 1; }
  echo "$sum"
}
dmg=$(digest "ELYTerminal-$version.dmg")
zip=$(digest "ELYTerminal-$version-windows-x86_64.zip")
zip=$(printf '%s' "$zip" | tr '[:lower:]' '[:upper:]')
out=target/package-managers/$version
rm -rf "$out"
mkdir -p "$out/Casks" "$out/winget"
fill() {
  sed -e "s/@VERSION@/$version/g" -e "s/@DMG_SHA256@/$dmg/g" -e "s/@ZIP_SHA256@/$zip/g" "$1" > "$2"
}
fill packaging/homebrew/elyterminal.rb "$out/Casks/elyterminal.rb"
for file in packaging/winget/*.yaml; do
  fill "$file" "$out/winget/${file##*/}"
done
echo "package-managers: $out holds the cask and winget's manifests for $version"
header="Authorization: Basic $(printf 'x-access-token:%s' "$(gh auth token)" | base64)"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
case "${1:-}" in
  "") exit 0 ;;
  --tap) ;;
  --winget)
    # A pull request from the user's fork of microsoft/winget-pkgs.
    login=$(gh api user --jq .login)
    gh repo fork microsoft/winget-pkgs --clone=false
    gh repo sync "$login/winget-pkgs"
    place=manifests/e/ELYOpenSource/ELYTerminal
    if gh api "repos/microsoft/winget-pkgs/contents/$place" --silent 2>/dev/null; then
      title="New version: ELYOpenSource.ELYTerminal version $version"
    else
      title="New package: ELYOpenSource.ELYTerminal version $version"
    fi
    git -c http.extraHeader="$header" clone -q --filter=blob:none --no-checkout --depth 1 \
      "https://github.com/$login/winget-pkgs.git" "$work"
    git -C "$work" sparse-checkout set --no-cone "/$place/"
    git -C "$work" checkout -q -b "elyterminal-$version"
    mkdir -p "$work/$place/$version"
    cp "$out"/winget/*.yaml "$work/$place/$version/"
    git -C "$work" add "$place/$version"
    git -C "$work" commit -q -m "$title"
    git -C "$work" -c http.extraHeader="$header" push -q origin "elyterminal-$version"
    gh pr create -R microsoft/winget-pkgs --head "$login:elyterminal-$version" --title "$title" \
      --body "ELYTerminal $version, a portable zip from https://github.com/$repo/releases/tag/v$version. Validated with \`winget validate\`."
    exit 0
    ;;
  *) echo "package-managers: use no option, --tap or --winget" >&2; exit 1 ;;
esac
tap=$work
git -c http.extraHeader="$header" clone -q https://github.com/ELY-Open-Source/homebrew-tap.git "$tap"
mkdir -p "$tap/Casks"
cp "$out/Casks/elyterminal.rb" "$tap/Casks/elyterminal.rb"
git -C "$tap" add Casks/elyterminal.rb
if git -C "$tap" diff --cached --quiet; then
  echo "package-managers: the tap already holds $version"
  exit 0
fi
git -C "$tap" commit -q -m "elyterminal $version"
git -C "$tap" -c http.extraHeader="$header" push -q origin HEAD
echo "package-managers: the tap holds elyterminal $version"
