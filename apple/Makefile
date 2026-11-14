.PHONY: build test generate open clean app device perf testflight

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

# Builds the iOS app (Release: Debug's unoptimised Swift makes scrolling and zooming feel slower
# than the real app), installs it on the first connected iPhone and opens it. Needs the iPhone paired
# (by cable, or on the same Wi-Fi once paired), unlocked and in Developer Mode.
# DEVICE=<udid> picks another one.
DEVICE ?= $(shell xcrun devicectl list devices 2>/dev/null | awk '/physical/ {for (i=1;i<=NF;i++) if ($$i ~ /^[0-9A-F]+-[0-9A-F]+$$/ && length($$i) == 25) {print $$i; exit}}')
DEVICE_BUILD = App/build-ios
DEVICE_APP = $(DEVICE_BUILD)/Build/Products/Release-iphoneos/TPApp-iOS.app

device: generate
	@test -n "$(DEVICE)" || (echo "No hay ningún iPhone conectado." && exit 1)
	xcodebuild -project App/TPApp.xcodeproj -scheme TPApp-iOS -configuration Release \
		-destination 'generic/platform=iOS' -derivedDataPath $(DEVICE_BUILD) -allowProvisioningUpdates -quiet build
	@# Over Wi-Fi the first connection is often reset; a second try goes through.
	xcrun devicectl device install app --device $(DEVICE) $(DEVICE_APP) \
		|| (sleep 2 && xcrun devicectl device install app --device $(DEVICE) $(DEVICE_APP))
	xcrun devicectl device process launch --device $(DEVICE) com.migueltaibo.tpcloud

# Measures the Photos tab on the connected iPhone (signed in already): hitch ratio while flinging
# and pinching, and how long photos take to open. TEST=<method> runs one test.
perf: generate
	@test -n "$(DEVICE)" || (echo "No hay ningún iPhone conectado." && exit 1)
	xcodebuild test -project App/TPApp.xcodeproj -scheme TPApp-iOS -configuration Release \
		-destination 'id=$(DEVICE)' -derivedDataPath App/build-perf -allowProvisioningUpdates \
		$(if $(TEST),-only-testing:TPAppUITests/PhotosPerformanceTests/$(TEST)) \
		| grep -E "PERF|Test Case.*(passed|failed)|error:"

# Archives the iPhone and Mac apps and uploads both to App Store Connect (TestFlight). Needs Xcode signed
# in with the developer account and the app created in App Store Connect with bundle id
# com.migueltaibo.tpcloud (iOS and macOS). Every upload needs a build number higher than the last: it
# defaults to the current date and time, BUILD=<n> overrides it. The marketing version is in Shared.xcconfig.
BUILD ?= $(shell date +%y%m%d%H%M)
# Fixed once, so the iPhone and the Mac uploads share a number.
BUILD := $(BUILD)
TEAM = $(shell awk -F= '/^DEVELOPMENT_TEAM/ {gsub(/ /, "", $$2); print $$2}' App/Config/Local.xcconfig 2>/dev/null || true)
TEAM_ID = $(or $(TEAM),$(shell awk '/DEVELOPMENT_TEAM:/ {print $$2; exit}' App/project.yml))
ARCHIVE = App/build-archive/TPApp-iOS.xcarchive
MAC_ARCHIVE = App/build-archive/TPApp-macOS.xcarchive

testflight: generate
	@test -n "$(TEAM_ID)" || (echo "Falta DEVELOPMENT_TEAM en App/Config/Local.xcconfig." && exit 1)
	@mkdir -p App/build-archive
	@printf '%s\n' '<?xml version="1.0" encoding="UTF-8"?>' \
		'<plist version="1.0"><dict>' \
		'<key>method</key><string>app-store-connect</string>' \
		'<key>destination</key><string>upload</string>' \
		'<key>teamID</key><string>$(TEAM_ID)</string>' \
		'<key>signingStyle</key><string>automatic</string>' \
		'</dict></plist>' > App/build-archive/ExportOptions.plist
	rm -rf $(ARCHIVE) $(MAC_ARCHIVE)
	xcodebuild -project App/TPApp.xcodeproj -scheme TPApp-iOS -configuration Release \
		-destination 'generic/platform=iOS' -archivePath $(ARCHIVE) -allowProvisioningUpdates \
		CURRENT_PROJECT_VERSION=$(BUILD) -quiet archive
	xcodebuild -exportArchive -archivePath $(ARCHIVE) -exportOptionsPlist App/build-archive/ExportOptions.plist \
		-allowProvisioningUpdates
	xcodebuild -project App/TPApp.xcodeproj -scheme TPApp-macOS -configuration Release \
		-destination 'generic/platform=macOS' -archivePath $(MAC_ARCHIVE) -allowProvisioningUpdates \
		CURRENT_PROJECT_VERSION=$(BUILD) -quiet archive
	xcodebuild -exportArchive -archivePath $(MAC_ARCHIVE) -exportOptionsPlist App/build-archive/ExportOptions.plist \
		-allowProvisioningUpdates
