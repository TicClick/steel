#!/usr/bin/env bash

set -eu -o pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: $0 <target>" && exit 1
fi

target=$1
root=$( cd "$( dirname "$0" )/../.." && pwd )
version=$( sed -n 's/^version = "\(.*\)"$/\1/p' "$root/Cargo.toml" | head -1 )
name="steel-macos-${target%%-*}"

work=$( mktemp -d )
trap 'rm -rf "$work"' EXIT

app="$work/steel.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
sed "s/@VERSION@/$version/g" "$root/packaging/macos/Info.plist" > "$app/Contents/Info.plist"
cp "$root/target/$target/release/steel" "$app/Contents/MacOS/steel"
cp "$root/media/icons/application.icns" "$app/Contents/Resources/steel.icns"

codesign --force --sign - "$app"
codesign --verify --strict "$app"

COPYFILE_DISABLE=1 tar --no-mac-metadata --no-xattrs -czf "$root/$name.app.tar.gz" -C "$work" steel.app

chmod 755 "$work"
ln -s /Applications "$work/Applications"
hdiutil create -quiet -ov -volname steel -srcfolder "$work" -format UDZO "$root/$name.dmg"
