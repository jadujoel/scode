#!/bin/bash

# !!!! make sure you gitignore .codesign and .env file !!!!!!

# Codesign requires some setup locally
# Before the github action can be run, you need to create a .p12 file and a .certSigningRequest file
# And then store them as secrets on github

# Step 1: Generate a CSR Using Keychain Access on macOS
# Open Keychain Access: Go to your Applications folder, then Utilities, and open Keychain Access.
# Access Certificate Assistant: From the Keychain Access menu, select Certificate Assistant > Request a Certificate From a Certificate Authority.
# Certificate Information:
# User Email Address: Enter your email address. This should be the same email associated with your Apple Developer account.
# Common Name: Enter your name or your company's name. This name will be associated with the certificate.
# CA Email Address: Leave this blank as you will be manually submitting the CSR to Apple.
# Request is: Choose "Saved to disk" to save your CSR file to your computer.
# save it in <this-repo>/.codesign/CertificateSigningRequest.certSigningRequest
# go to https://developer.apple.com/account/resources/certificates/list
# press +
# Choose "Developer ID Application" and press "Continue"
# press "Choose File" and select the .certSigningRequest file
# press "Continue"
# press "Download" and save the .cer file in <this-repo>/.codesign/developerID_application.cer

# Step 2: Convert the .cer file to a .p12 file
# Open Keychain Access
# Import the .cer file: From the Keychain Access menu, select File > Import Items, then navigate to the .cer file you downloaded from Apple and click Open.
# Enter your password: You may be prompted to enter your password to install the certificate. Enter the same password you use to log into your computer.
# Export the .p12 file: In Keychain Access, select the "My Certificates" category in the left sidebar.
# Generate a password: `openssl rand -base64 12 > .codesign/P12_PASSWORD`
# Next, select the certificate you want to export, click File > Export Items,
# and save the file in the .p12 format.
# save it in <this-repo>/.codesign/developerID_application.p12
# paste the password from the .codesign/P12_PASSWORD file and click OK
# enter your computer password and press allow or always allow
# you should now have a developerID_application.p12 file in the .codesign folder

# Step2:
# Generate an API key from the App Store Connect portal.
# You'll need the Key ID, Issuer ID, and the API Key file (.p8 file).
# Get it at: https://appstoreconnect.apple.com/access/integrations/api
# then download the key to <this-repo>/.codesign/AuthKey_<SOME_STRING>.p8

# Step3:
# Create a .env file in the root of the project
# TEAM_ID=YOUR_TEAM_ID
# APP_NAME=YOUR_APP_NAME
# BUNDLE_ID=YOUR_BUNDLE_ID
# DEVELOPER_ID=YOUR_DEVELOPER_ID
# YOUR_NAME=YOUR_NAME
# API_KEY_ID=YOUR_API_KEY
# API_KEY_ISSUER_ID=YOUR_API_KEY
# P12_PASSWORD=YOUR_P12_PASSWORD

# Step4
# run bash codesign.sh --setup
# this will update the .env file with the base64 encoded .p12 and .p8 files

# Step5
# run `bash codesign.sh``
# this will codesign the app and create a .app bundle

# Step6
# enjoy

ENV_FILE=".env"

# target is provided by the user as the first argument
# if not provided, the default value is "scode"
# this is needed since rust puts the binary in a folder with the name of the project
# example target/release/scode
# or target/aarch64-apple-darwin/release/scode
TARGET=${1:-"release"}

echo "Target: $TARGET"

mkdir -p .codesign

if [ -f .env ]; then
  source .env
fi
echo "Setting up codesigning"
if [ -z "$P12_PASSWORD" ]; then
  echo "P12_PASSWORD is not set"
  exit 1
fi
# Check if P12_BASE64 variable exists in the .env file silently
if [ -z "$P12_BASE64" ]; then
  echo "P12_BASE64 variable not found in $ENV_FILE. Adding it now."
  cd .codesign || exit 1
  base64 < developerID_application.p12 > developerID_application.p12.base64
  cd ..
  # Path to your base64-encoded .p12 file
  P12_BASE64_FILE=".codesign/developerID_application.p12.base64"
  # Read the content of the base64-encoded .p12 file
  P12_CONTENT=$(cat "$P12_BASE64_FILE")
  # Append the P12_BASE64 variable and its content to the .env file
  echo "P12_BASE64=\"$P12_CONTENT\"" >> "$ENV_FILE"
else
  echo "P12_BASE64 variable already exists in $ENV_FILE."
fi
if [ -z "$API_KEY_BASE64" ]; then
  echo "API_KEY_BASE64 variable not found in $ENV_FILE. Adding it now."
  cd .codesign || exit 1
  base64 < "AuthKey_$API_KEY_ID.p8" > "AuthKey_$API_KEY_ID.p8.base64"
  cd ..
  # Path to your base64-encoded .p12 file
  API_KEY_BASE64_FILE=".codesign/AuthKey_$API_KEY_ID.p8.base64"
  # Read the content of the base64-encoded .p12 file
  API_KEY_CONTENT=$(cat "$API_KEY_BASE64_FILE")
  # Append the P12_BASE64 variable and its content to the .env file
  echo "API_KEY_BASE64=\"$API_KEY_CONTENT\"" >> "$ENV_FILE"
else
  echo "API_KEY_BASE64 variable already exists in $ENV_FILE."
fi

if [ -f .env ]; then
  source .env
fi

if [ -z "$TEAM_ID" ]; then
    echo "TEAM_ID is not set"
    exit 1
fi

if [ -z "$APP_NAME" ]; then
    echo "APP_NAME is not set"
    exit 1
fi

if [ -z "$BUNDLE_ID" ]; then
    echo "BUNDLE_ID is not set"
    exit 1
fi

if [ -z "$DEVELOPER_ID" ]; then
    echo "DEVELOPER_ID is not set"
    exit 1
fi

if [ -z "$YOUR_NAME" ]; then
    echo "YOUR_NAME is not set"
    exit 1
fi

if [ -z "$API_KEY_ID" ]; then
    echo "API_KEY_ID is not set"
    exit 1
fi

if [ -z "$API_KEY_ISSUER_ID" ]; then
    echo "API_KEY_ISSUER_ID is not set"
    exit 1
fi

if [ -z "$P12_PASSWORD" ]; then
    echo "P12_PASSWORD is not set"
    exit 1
fi

if [ -z "$P12_BASE64" ]; then
    echo "P12_BASE64 is not set"
    exit 1
fi

if [ -z "$API_KEY_BASE64" ]; then
    echo "API_KEY_BASE64 is not set"
    exit 1
fi

INPUT_PATH="target/$TARGET/$APP_NAME"
if [ ! -f "$INPUT_PATH" ]; then
  echo "The file $INPUT_PATH does not exist"
  exit 1
fi

echo "Creating an Entitlements file..."
ENTITLEMENTS=".codesign/entitlements.plist"
touch .env
cat << EOF > $ENTITLEMENTS
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    <!-- Add additional entitlements here -->
</dict>
</plist>
EOF

## if api key file doesnt exist, create it from env variable
API_KEY_PATH=".codesign/AuthKey_$API_KEY_ID.p8"
if [ ! -f "$API_KEY_PATH" ]; then
  echo "Creating API key file from base64..."
  echo "$API_KEY_BASE64" | base64 --decode > "$API_KEY_PATH"
fi

P12_FILE_PATH=.codesign/developerID_application.p12
if [ ! -f $P12_FILE_PATH ]; then
  echo "Creating .p12 file from base64..."
  echo "$P12_BASE64" | base64 --decode > $P12_FILE_PATH
fi

KEYCHAIN_PATH=~/Library/Keychains/login.keychain-db

echo "Creating a temporary keychain..."
security create-keychain -p "" temp.keychain-db
security default-keychain -s temp.keychain-db

echo "Unlocking the keychain..."
security unlock-keychain -p "" temp.keychain-db

echo "Importing the .p12 file into the temporary keychain..."
security import $P12_FILE_PATH -k temp.keychain-db -P "$P12_PASSWORD" -T /usr/bin/codesign

echo "Adding the temporary keychain to the search list..."
security list-keychains -s temp.keychain-db $KEYCHAIN_PATH

echo "Setting key partition list to allow codesign to access the keychain..."
security set-key-partition-list -S apple-tool:,apple: -s -k "" temp.keychain-db

echo "Signing the application..."
DEVELOPER_ID="Developer ID Application: $YOUR_NAME ($TEAM_ID)"
codesign --sign "$DEVELOPER_ID" --entitlements "$ENTITLEMENTS" --options runtime --timestamp --force "$INPUT_PATH"

# Note: 'notarytool submit' with '--wait' option waits for notarization to complete.
# If notarization is successful, the tool will output a success message.
# If there is an error, it will provide details on the failure.

echo "Creating an app bundle..."

mkdir -p "$APP_NAME.app/Contents/MacOS"
cp "target/$TARGET/$APP_NAME" "$APP_NAME.app/Contents/MacOS/$APP_NAME"

touch "$APP_NAME.app/Contents/Info.plist"
BUNDLE_VERSION="1.0"

printf "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
<dict>
    <key>CFBundleExecutable</key>
    <string>%s</string>
    <key>CFBundleIdentifier</key>
    <string>%s</string>
    <key>CFBundleName</key>
    <string>%s</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>CFBundleShortVersionString</key>
    <string>%s</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.9</string>
    <key>LSMultipleInstancesProhibited</key>
    <true/>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2023. All rights reserved.</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>com.apple.security.cs.disable-library-validation</key>
    <true/>
    <key>com.apple.security.cs.disable-executable-page-protection</key>
    <true/>
</dict>
</plist>
" "${APP_NAME}" "${BUNDLE_ID}" "${APP_NAME}" "${BUNDLE_VERSION}" > "$APP_NAME.app/Contents/Info.plist"

echo "Codesigning the app bundle..."
INPUT_PATH="$APP_NAME.app"
codesign --deep --sign "$DEVELOPER_ID" --entitlements "$ENTITLEMENTS" --options runtime --timestamp --force "$INPUT_PATH"

echo "Creating a ZIP archive for notarization..."
OUTPUT_PATH="target/$TARGET/$APP_NAME.zip"
ditto -c -k --keepParent "$INPUT_PATH" "$OUTPUT_PATH"

echo "Uploading the app for notarization..."
xcrun notarytool submit "$OUTPUT_PATH" --key-id "$API_KEY_ID" --issuer "$API_KEY_ISSUER_ID" --key "$API_KEY_PATH" --wait

echo "Stapling the app..."
xcrun stapler staple "$INPUT_PATH"

echo "Removing the temporary keychain..."
security delete-keychain temp.keychain-db

echo "Codesigning complete!"
exit 0
