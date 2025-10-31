#!/bin/bash

# Test script for the watch mode functionality

echo "Testing Pact Stub Server Watch Mode"
echo "===================================="

# Create a temporary test pact file
TEST_PACT_FILE="/tmp/test-watch-pact.json"
cat > "$TEST_PACT_FILE" << 'EOF'
{
  "consumer": {
    "name": "test-consumer"
  },
  "provider": {
    "name": "test-provider"
  },
  "interactions": [
    {
      "description": "A test interaction",
      "request": {
        "method": "GET",
        "path": "/test"
      },
      "response": {
        "status": 200,
        "headers": {
          "Content-Type": "application/json"
        },
        "body": {
          "message": "Hello from initial pact!"
        }
      }
    }
  ],
  "metadata": {
    "pactSpecification": {
      "version": "2.0.0"
    }
  }
}
EOF

echo "Created test pact file at: $TEST_PACT_FILE"
echo ""
echo "Starting pact-stub-server in watch mode..."
echo "The server will watch for changes to the pact file and reload automatically."
echo ""
echo "To test:"
echo "1. Start the server (this script will start it)"
echo "2. In another terminal, run: curl http://localhost:8080/test"
echo "3. Modify the pact file and curl again to see changes"
echo ""
echo "Press Ctrl+C to stop the server"
echo ""

# Run the pact-stub-server with watch mode
cargo run --bin pact-stub-server -- --file "$TEST_PACT_FILE" --port 8080 --watch --loglevel info