#!/bin/bash

# CD into the directory this script is installed in
SCRIPT=$(readlink -f "$0")
SCRIPT_DIR=$(dirname "$SCRIPT")
cd $SCRIPT_DIR

# Set default values
MODE=${1:-upgrade} # MODE is either install, reinstall or upgrade
NETWORK=${2:-ic} # NETWORK is either ic or local

# ANSI color codes
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "Deploying bot with the following settings:"
echo "Mode: $MODE (default: upgrade)"
echo "Network: $NETWORK (default: ic)"
echo ""

# Read the OpenChat public key from the website
OC_PUBLIC_KEY=$(curl -s https://oc.app/public-key)

# Build the bot install args
ARGS="(record { oc_public_key = \"$OC_PUBLIC_KEY\" } )"

# Generate Cargo.lock file first
echo "Generating Cargo.lock file..."
cd ..
cargo build --target wasm32-unknown-unknown || exit 1
cd $SCRIPT_DIR

if [ "$NETWORK" = "ic" ]; then
    echo "Deploying to ic network..."
    
    # Try to get the canister ID to check if it exists
    CANISTER_ID=$(dfx canister id kongbot --network ic 2>/dev/null)
    
    if [ $? -eq 0 ] && [ ! -z "$CANISTER_ID" ]; then
        echo "Canister exists. Attempting upgrade..."
        # Try to upgrade first
        if ! dfx deploy kongbot --ic --argument "$ARGS" --mode upgrade; then
            echo "Upgrade failed. The canister might have expired. Creating a new one..."
            # If upgrade fails, create a new canister
            dfx deploy kongbot --ic --argument "$ARGS" --mode install
        fi
    else
        echo "Canister does not exist. Creating a new one..."
        # Create a new canister
        dfx deploy kongbot --ic --argument "$ARGS" --mode install
    fi
    
    # Get the canister ID after deployment
    CANISTER_ID=$(dfx canister id kongbot --network ic)
    
    echo ""
    echo "Bot successfully deployed to ic network!"
    echo -e "${GREEN}Bot URL: https://$CANISTER_ID.raw.icp0.io/${NC}"
    echo -e "Note: The bot is accessible via the URL above. Test it out using: ${BLUE}curl https://$CANISTER_ID.raw.icp0.io/${NC}"
    echo "Warning: ic canisters expire after 20 minutes. You may need to redeploy after expiration."
    echo ""
else
    # For local deployment, use the normal process
    ./utils/deploy.sh kongbot OnchainBot $MODE "$ARGS" $NETWORK
fi