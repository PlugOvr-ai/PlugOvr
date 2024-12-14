cargo build --release --features metal
cargo bundle --release --features metal

# Add privacy permissions after bundle creation
/usr/libexec/PlistBuddy -c "Add :NSInputMonitoringUsageDescription string 'PlugOvr needs input monitoring permissions to function properly.'" \
    "target/release/bundle/osx/PlugOvr.app/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :NSAppleEventsUsageDescription string 'PlugOvr needs automation permissions to interact with other applications.'" \
    "target/release/bundle/osx/PlugOvr.app/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :NSScreenCaptureUsageDescription string 'PlugOvr needs screen recording permissions to capture and display your screen.'" \
    "target/release/bundle/osx/PlugOvr.app/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :NSAccessibilityUsageDescription string 'PlugOvr needs accessibility permissions to function properly.'" \
    "target/release/bundle/osx/PlugOvr.app/Contents/Info.plist"

xattr -cr target/release/bundle/osx/PlugOvr.app
codesign -s $DEV_ID_APP --entitlements entitlements.plist --deep --force --options runtime target/release/bundle/osx/PlugOvr.app/Contents/MacOS/*
codesign -s $DEV_ID_APP --entitlements entitlements.plist --deep --force --options runtime target/release/bundle/osx/PlugOvr.app --entitlements entitlements.plist

#ditto -c -k --keepParent "target/release/bundle/osx/PlugOvr.app" "PlugOvr.zip"

# Create a temporary directory for DMG contents
TMP_DMG_DIR="tmp_dmg"
mkdir -p "${TMP_DMG_DIR}"

# Create Applications folder symlink
ln -s /Applications "${TMP_DMG_DIR}/Applications"

# Copy the app to the temporary directory
cp -r "target/release/bundle/osx/PlugOvr.app" "${TMP_DMG_DIR}/"

# Create DMG with background and positioning
hdiutil create -volname "PlugOvr" \
    -srcfolder "${TMP_DMG_DIR}" \
    -ov -format UDZO \
    -fs HFS+ \
    -size 200m \
    "target/PlugOvr.dmg"

# Clean up
rm -rf "${TMP_DMG_DIR}"

# Get version from Cargo.toml
VERSION=$(grep '^version =' Cargo.toml | head -n1 | cut -d'"' -f2)

# Rename DMG with version
mv target/PlugOvr.dmg "target/PlugOvr_${VERSION}_aarch64.dmg"

xcrun notarytool submit target/PlugOvr_${VERSION}_aarch64.dmg --apple-id $APPLE_ID --password $APPLE_ID_PASSWORD --team-id $TEAM_ID --wait

xcrun stapler staple target/PlugOvr_${VERSION}_aarch64.dmg

aws s3 cp target/PlugOvr_${VERSION}_aarch64.dmg s3://plugovr.ai/

echo "{\"version\": \"${VERSION}\"}" > target/latest_osx_aarch64_version.json
aws s3 cp target/latest_osx_aarch64_version.json s3://plugovr.ai/latest_osx_aarch64_version.json

#codesign -s "Developer ID Application: %(id)" --force --options runtime  ./target/release/bundle/osx/RustDesk.app/Contents/MacOS/*
#codesign -s "Developer ID Application: %(id)" --force --options runtime  ./target/release/bundle/osx/RustDesk.app
