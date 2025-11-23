#!/bin/bash
# Test script to check if TUI logs are captured properly

# Reset terminal
reset

echo "=== Testing CDKTR TUI ==="
echo "Starting TUI for 2 seconds..."
echo "If you see log messages below the TUI, the fix didn't work."
echo "Press 2 to switch to Admin tab to see captured logs."
echo ""
sleep 1

# Run TUI with timeout
timeout 2 ./target/debug/cdktr-cli ui 2>&1 || true

# Reset terminal again
reset

echo "=== Test Complete ==="
echo "Did you see any log messages printed to the terminal? (y/n)"
echo "If NO: Fix is working! Logs are captured in memory."
echo "If YES: Fix failed, logs are still being printed to stdout/stderr."
