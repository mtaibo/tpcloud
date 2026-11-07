.PHONY: build test generate open clean app device

build:
	swift build

test:
	swift test

generate:
	cd App && xcodegen generate

open: generate
	open App/TPApp.xcodeproj

clean:
	swift package clean
	rm -rf App/TPApp.xcodeproj

# Builds the macOS app (ad-hoc signed, runs without a paid Apple Developer account),
# installs it to /Applications and launches it.
app: generate
	cd App && xcodebuild -project TPApp.xcodeproj -scheme TPApp-macOS -configuration Release \
		-derivedDataPath build CODE_SIGN_IDENTITY="-" CODE_SIGNING_REQUIRED=NO CODE_SIGNING_ALLOWED=YES build
	rm -rf /Applications/TPCloud.app
	cp -R App/build/Build/Products/Release/TPApp-macOS.app /Applications/TPCloud.app
	open /Applications/TPCloud.app

# Builds the iOS app, installs it on the first connected iPhone and opens it. Needs the iPhone paired
# (by cable, or on the same Wi-Fi once paired), unlocked and in Developer Mode.
# DEVICE=<udid> picks another one.
DEVICE ?= $(shell xcrun devicectl list devices 2>/dev/null | awk '/physical/ {for (i=1;i<=NF;i++) if ($$i ~ /^[0-9A-F]+-[0-9A-F]+$$/ && length($$i) == 25) {print $$i; exit}}')
DEVICE_BUILD = App/build-ios
DEVICE_APP = $(DEVICE_BUILD)/Build/Products/Debug-iphoneos/TPApp-iOS.app

device: generate
	@test -n "$(DEVICE)" || (echo "No hay ningún iPhone conectado." && exit 1)
	xcodebuild -project App/TPApp.xcodeproj -scheme TPApp-iOS -configuration Debug \
		-destination 'generic/platform=iOS' -derivedDataPath $(DEVICE_BUILD) -allowProvisioningUpdates -quiet build
	@# Over Wi-Fi the first connection is often reset; a second try goes through.
	xcrun devicectl device install app --device $(DEVICE) $(DEVICE_APP) \
		|| (sleep 2 && xcrun devicectl device install app --device $(DEVICE) $(DEVICE_APP))
	xcrun devicectl device process launch --device $(DEVICE) com.migueltaibo.tpapp
