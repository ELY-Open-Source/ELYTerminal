#!/bin/sh
# mac-app.sh: ELYTerminal.app in target/release and its disk image, signed ad hoc, or by SIGN_IDENTITY and notarized.
set -eu
cd "$(dirname "$0")/.."
sign=${SIGN_IDENTITY:-}
if [ -n "$sign" ]; then
  [ -s "${NOTARY_KEY:-}" ] && [ -n "${NOTARY_KEY_ID:-}" ] && [ -n "${NOTARY_ISSUER:-}" ] || {
    echo "mac-app: SIGN_IDENTITY needs NOTARY_KEY (a .p8 file), NOTARY_KEY_ID and NOTARY_ISSUER" >&2
    exit 1
  }
  security find-identity -v -p codesigning | grep -q -F "\"$sign\"" || {
    echo "mac-app: no valid identity \"$sign\" in the keychains" >&2
    exit 1
  }
fi
cargo build --release --locked
version=$(cargo pkgid)
version=${version##*[#@]}
case $version in [0-9]*) ;; *) echo "mac-app: cargo pkgid names no version" >&2; exit 1 ;; esac
out=${CARGO_TARGET_DIR:-target}/release
app=$out/ELYTerminal.app
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cargo run --release --locked --quiet --example icons -- \
  iconset assets/icons/elyterminal-macos.svg "$work/ELYTerminal.iconset"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$out/elyterminal" "$app/Contents/MacOS/"
iconutil -c icns "$work/ELYTerminal.iconset" -o "$app/Contents/Resources/ELYTerminal.icns"
cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleDisplayName</key><string>ELYTerminal</string>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key><string>Folder</string>
      <key>CFBundleTypeRole</key><string>Viewer</string>
      <key>LSHandlerRank</key><string>Alternate</string>
      <key>LSItemContentTypes</key><array><string>public.folder</string></array>
    </dict>
  </array>
  <key>CFBundleExecutable</key><string>elyterminal</string>
  <key>CFBundleIconFile</key><string>ELYTerminal</string>
  <key>CFBundleIdentifier</key><string>com.zacharyzhang.elyterminal</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>ELYTerminal</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF
plutil -lint -s "$app/Contents/Info.plist"
if [ -n "$sign" ]; then
  codesign --force --options runtime --timestamp --sign "$sign" "$app"
else
  codesign --force --sign - "$app"
fi
mkdir "$work/disk"
ditto "$app" "$work/disk/ELYTerminal.app"
ln -s /Applications "$work/disk/Applications"
dmg=$out/ELYTerminal-$version.dmg
hdiutil create -quiet -volname ELYTerminal -srcfolder "$work/disk" -ov -format UDZO "$dmg"
if [ -n "$sign" ]; then
  codesign --force --timestamp --sign "$sign" "$dmg"
  notary() { xcrun notarytool "$@" --key "$NOTARY_KEY" --key-id "$NOTARY_KEY_ID" --issuer "$NOTARY_ISSUER"; }
  notary submit "$dmg" --wait --output-format json > "$work/notary.json" || true
  status=$(plutil -extract status raw -o - "$work/notary.json" 2>/dev/null || echo none)
  echo "mac-app: notarization $status"
  if [ "$status" != Accepted ]; then
    cat "$work/notary.json" >&2
    id=$(plutil -extract id raw -o - "$work/notary.json" 2>/dev/null) && notary log "$id" >&2
    exit 1
  fi
  xcrun stapler staple "$dmg"
  spctl --assess --type open --context context:primary-signature --verbose "$dmg"
fi
echo "mac-app: $app"
echo "mac-app: $dmg"
