#!/bin/bash
#
# TurboMCP Studio Sampling & Elicitation Test Runner
#
# This script orchestrates comprehensive testing of sampling/elicitation features.
# It can run individual scenarios or the full test suite.
#
# Usage:
#   ./run_sampling_tests.sh [scenario]
#
# Scenarios:
#   simple      - Basic sampling request
#   complex     - Multi-message with system prompt
#   preferences - Model preference testing
#   image       - Image content sampling
#   elicitation - Text input elicitation
#   schema      - Complex schema validation
#   all         - Run all scenarios (default)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCENARIO="${1:-simple}"
SERVER_SCRIPT="test_sampling_elicitation_server.py"
LOG_FILE="/tmp/turbomcp_studio_test_$(date +%Y%m%d_%H%M%S).log"

# Banner
echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  TurboMCP Studio Sampling & Elicitation Test Suite          ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Check prerequisites
echo -e "${BLUE}🔍 Checking prerequisites...${NC}"

if [ ! -f "$SERVER_SCRIPT" ]; then
    echo -e "${RED}❌ Test server not found: $SERVER_SCRIPT${NC}"
    exit 1
fi

if ! command -v python3 &> /dev/null; then
    echo -e "${RED}❌ python3 not found${NC}"
    exit 1
fi

if [ -z "$OPENAI_API_KEY" ]; then
    echo -e "${YELLOW}⚠️  OPENAI_API_KEY not set - LLM integration will fail${NC}"
    echo -e "${YELLOW}   Set with: export OPENAI_API_KEY='your-key-here'${NC}"
fi

echo -e "${GREEN}✅ Prerequisites OK${NC}"
echo ""

# Display test info
echo -e "${BLUE}📋 Test Configuration${NC}"
echo "  Scenario: $SCENARIO"
echo "  Server: $SERVER_SCRIPT"
echo "  Log file: $LOG_FILE"
echo ""

# Instructions
echo -e "${YELLOW}📝 Manual Testing Instructions:${NC}"
echo ""
echo "1. This script will start the test MCP server"
echo "2. In TurboMCP Studio, add the test server with these settings:"
echo "   - Name: 'Sampling Test Server'"
echo "   - Transport: stdio"
echo "   - Command: python3"
echo "   - Args: $(pwd)/$SERVER_SCRIPT $SCENARIO"
echo ""
echo "3. Connect to the server in TurboMCP Studio"
echo "4. The server will automatically send test requests"
echo "5. Observe the UI behavior and verify against checklist"
echo ""
echo -e "${BLUE}Expected Behavior:${NC}"

case "$SCENARIO" in
    simple)
        echo "  - Sampling approval modal should appear"
        echo "  - Display single user message: 'What is the capital of France?'"
        echo "  - Show maxTokens: 100"
        echo "  - Cost/token estimates displayed"
        echo "  - Approve button sends to LLM"
        ;;
    complex)
        echo "  - Sampling approval modal should appear"
        echo "  - Display 3-message conversation history"
        echo "  - Show system prompt about travel assistant"
        echo "  - Show temperature: 0.7, maxTokens: 200"
        echo "  - Show stop sequences: ['END', 'STOP']"
        ;;
    preferences)
        echo "  - Sampling approval modal should appear"
        echo "  - Display model preferences section"
        echo "  - Show cost priority: 0.8 (high)"
        echo "  - Show speed priority: 0.5 (medium)"
        echo "  - Show intelligence priority: 0.3 (low)"
        echo "  - Show hints: gpt-4o-mini, gpt-4o, claude-3-haiku, claude-3-5-sonnet"
        echo "  - Model selection should prefer cheaper models"
        ;;
    image)
        echo "  - Sampling approval modal should appear"
        echo "  - Display image content (small red pixel)"
        echo "  - Display follow-up text message"
        echo "  - Image should be rendered or show data preview"
        ;;
    elicitation)
        echo "  - Elicitation dialog should appear"
        echo "  - Display message: 'Please enter your preferred model name'"
        echo "  - Show text input field with validation"
        echo "  - Accept/Decline/Cancel buttons present"
        ;;
    schema)
        echo "  - Elicitation dialog should appear"
        echo "  - Display message: 'Configure your LLM sampling preferences'"
        echo "  - Show enum dropdown for model selection"
        echo "  - Show number input for temperature (0.0-1.0)"
        echo "  - Show integer input for maxTokens (1-4096)"
        echo "  - Show boolean checkbox for streaming"
        echo "  - Show optional email field with format validation"
        ;;
    all)
        echo "  - All scenarios will run sequentially with 2-second delay"
        echo "  - Server will send 6 different test requests"
        echo "  - Verify each scenario's expected behavior"
        ;;
    *)
        echo -e "${RED}Unknown scenario: $SCENARIO${NC}"
        echo "Valid scenarios: simple, complex, preferences, image, elicitation, schema, all"
        exit 1
        ;;
esac

echo ""
echo -e "${YELLOW}Press Enter to start the test server...${NC}"
read

# Start test server
echo -e "${GREEN}🚀 Starting test server...${NC}"
echo ""

# Run the server and log output
python3 "$SERVER_SCRIPT" "$SCENARIO" 2>&1 | tee "$LOG_FILE"

# Print summary
echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  Test Run Complete                                           ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
echo -e "${BLUE}📊 Test Results${NC}"
echo "  Log file: $LOG_FILE"
echo ""
echo -e "${YELLOW}📝 Manual Verification Checklist:${NC}"
echo ""
echo "UI Tests:"
echo "  [ ] Sampling approval modal appeared"
echo "  [ ] All message content was visible"
echo "  [ ] System prompt was displayed (if present)"
echo "  [ ] Model preferences were shown (if present)"
echo "  [ ] Cost/token estimates were calculated"
echo "  [ ] Protocol Inspector link worked"
echo ""
echo "Functional Tests:"
echo "  [ ] Approve button sent request to LLM"
echo "  [ ] Response was received and formatted correctly"
echo "  [ ] Reject button sent error response"
echo "  [ ] Edit request modified content correctly"
echo "  [ ] Quick response templates worked"
echo ""
echo "Protocol Tests:"
echo "  [ ] JSON-RPC 2.0 format was correct"
echo "  [ ] Request/response ID correlation worked"
echo "  [ ] Capability declaration included sampling"
echo "  [ ] camelCase field names used throughout"
echo ""
echo -e "${BLUE}Next Steps:${NC}"
echo "  1. Review the log file for errors"
echo "  2. Check TurboMCP Studio logs for backend issues"
echo "  3. Verify Protocol Inspector shows all messages"
echo "  4. Document any bugs or improvements needed"
echo ""
