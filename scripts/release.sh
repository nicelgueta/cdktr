#!/bin/bash

# Script to bump patch versions of changed crates and the workspace
# Only runs when on a release branch (release/*)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get current branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

# Check if we're on a release branch
if [[ ! "$CURRENT_BRANCH" =~ ^release/.* ]]; then
    echo -e "${YELLOW}WARN: This script only runs on release branches (release/*)${NC}"
    echo -e "${YELLOW}Current branch: $CURRENT_BRANCH${NC}"
    exit 0
fi

echo -e "${GREEN}✓ On release branch: $CURRENT_BRANCH${NC}"

# Get the last commit that was pushed to remote
REMOTE_BRANCH="origin/$CURRENT_BRANCH"

# Check if remote branch exists
if ! git rev-parse --verify "$REMOTE_BRANCH" &>/dev/null; then
    echo -e "${YELLOW}Warning: Remote branch $REMOTE_BRANCH does not exist.${NC}"
    echo -e "${YELLOW}Comparing against origin/main instead.${NC}"
    REMOTE_BRANCH="origin/main"
fi

# Function to bump patch version in a Cargo.toml file
bump_patch_version() {
    local file=$1
    local current_version=$(awk -F ' = ' '$1 ~ /^version/ { gsub(/["]/, "", $2); print $2; exit }' "$file")

    if [[ -z "$current_version" ]]; then
        echo -e "${RED}Error: Could not find version in $file${NC}"
        return 1
    fi

    # Parse version (expecting semver format: major.minor.patch)
    IFS='.' read -r major minor patch <<< "$current_version"

    # Increment patch version
    new_patch=$((patch + 1))
    local new_version="$major.$minor.$new_patch"

    # Update the version in the file
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        sed -i '' "s/^version = \"$current_version\"/version = \"$new_version\"/" "$file"
    else
        # Linux
        sed -i "s/^version = \"$current_version\"/version = \"$new_version\"/" "$file"
    fi

    echo -e "${GREEN}  $current_version → $new_version${NC}"
}

# Check workspace Cargo.toml for changes
echo ""
echo "Checking workspace version..."

WORKSPACE_CHANGED=false
if git diff --name-only "$REMOTE_BRANCH"..HEAD | grep -q "^Cargo.toml$"; then
    WORKSPACE_CHANGED=true
fi

# Check if any crate has changed
CRATES_DIR="crates"
CHANGED_CRATES=()

echo ""
echo "Checking crates for changes since last push to $REMOTE_BRANCH..."

for crate_dir in "$CRATES_DIR"/*; do
    if [[ -d "$crate_dir" ]]; then
        crate_name=$(basename "$crate_dir")

        # Check if any files in this crate directory have changed
        if git diff --name-only "$REMOTE_BRANCH"..HEAD | grep -q "^$crate_dir/"; then
            CHANGED_CRATES+=("$crate_name")
            echo -e "${YELLOW}  ✓ $crate_name - has changes${NC}"
        else
            echo -e "  - $crate_name - no changes"
        fi
    fi
done

# If workspace Cargo.toml changed or any crate changed, bump workspace version
if [[ "$WORKSPACE_CHANGED" == true ]] || [[ ${#CHANGED_CRATES[@]} -gt 0 ]]; then
    echo ""
    echo "Bumping workspace version in Cargo.toml..."
    bump_patch_version "Cargo.toml"
else
    echo ""
    echo -e "${GREEN}No changes detected. No version bumps needed.${NC}"
    exit 0
fi

# Bump versions for changed crates
if [[ ${#CHANGED_CRATES[@]} -gt 0 ]]; then
    echo ""
    echo "Bumping versions for changed crates..."

    for crate_name in "${CHANGED_CRATES[@]}"; do
        crate_toml="$CRATES_DIR/$crate_name/Cargo.toml"
        if [[ -f "$crate_toml" ]]; then
            echo "  $crate_name:"
            bump_patch_version "$crate_toml"
        fi
    done
fi

echo ""
echo -e "${GREEN}✓ Version bumping complete!${NC}"
echo ""
echo "Changed files:"
git diff --name-only | grep "Cargo.toml"

echo ""
# Update Cargo.lock to reflect version changes
echo "Updating Cargo.lock..."
cargo update --workspace

echo ""
# Add changed Cargo.toml and Cargo.lock files and commit
git add Cargo.toml Cargo.lock
for crate_name in "${CHANGED_CRATES[@]}"; do
    crate_toml="$CRATES_DIR/$crate_name/Cargo.toml"
    if [[ -f "$crate_toml" ]]; then
        git add "$crate_toml"
    fi
done

# Get new main version from Cargo.toml
NEW_MAIN_VERSION=$(awk -F ' = ' '$1 ~ /^version/ { gsub(/["]/, "", $2); print $2; exit }' Cargo.toml)

git commit -m "release version $NEW_MAIN_VERSION"

git push --set-upstream origin $CURRENT_BRANCH
