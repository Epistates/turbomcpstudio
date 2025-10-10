#!/usr/bin/env python3
"""
TurboMCP Studio Sampling & Elicitation Test Server

This MCP server implements comprehensive server-side sampling and elicitation
scenarios for testing TurboMCP Studio's HITL capabilities.

Features:
- Sampling requests (text, image, multi-message, model preferences)
- Elicitation requests (all schema types, validation)
- Configurable test scenarios
- Protocol compliance validation
- Detailed logging for debugging

Usage:
    python test_sampling_elicitation_server.py [scenario]

Scenarios:
    simple     - Basic sampling request
    complex    - Multi-message with system prompt
    preferences - Model preference testing
    image      - Image content sampling
    elicitation - Text input elicitation
    schema     - Complex schema validation
    all        - Run all scenarios sequentially
"""

import asyncio
import json
import sys
import base64
import logging
from typing import Any, Dict, List, Optional
from datetime import datetime

# Configure logging
logging.basicConfig(
    level=logging.DEBUG,
    format='[%(asctime)s] %(levelname)s: %(message)s',
    handlers=[
        logging.FileHandler('/tmp/mcp_test_server.log'),
        logging.StreamHandler(sys.stderr)
    ]
)
logger = logging.getLogger(__name__)


class McpMessage:
    """MCP JSON-RPC 2.0 message builder"""

    def __init__(self, jsonrpc: str = "2.0", id: Optional[str] = None,
                 method: Optional[str] = None, params: Optional[Dict] = None,
                 result: Optional[Any] = None, error: Optional[Dict] = None):
        self.jsonrpc = jsonrpc
        self.id = id
        self.method = method
        self.params = params or {}
        self.result = result
        self.error = error

    def to_dict(self):
        msg = {"jsonrpc": self.jsonrpc}
        if self.id is not None:
            msg["id"] = self.id
        if self.method:
            msg["method"] = self.method
            msg["params"] = self.params
        if self.result is not None:
            msg["result"] = self.result
        if self.error:
            msg["error"] = self.error
        return msg


class SamplingElicitationTestServer:
    """
    MCP Test Server for Sampling & Elicitation

    This server sends various sampling and elicitation requests to test
    the TurboMCP Studio client's HITL capabilities.
    """

    def __init__(self, scenario: str = "simple"):
        self.scenario = scenario
        self.message_id_counter = 0
        self.initialized = False
        self.test_results = []

        # Test scenarios
        self.scenarios = {
            "simple": self.test_simple_sampling,
            "complex": self.test_complex_sampling,
            "preferences": self.test_model_preferences,
            "image": self.test_image_content,
            "elicitation": self.test_simple_elicitation,
            "schema": self.test_complex_schema,
            "all": self.test_all_scenarios,
        }

        logger.info(f"Test server initialized with scenario: {scenario}")

    def next_id(self) -> str:
        """Generate unique message ID"""
        self.message_id_counter += 1
        return f"test-{self.message_id_counter}"

    async def send_message(self, message: McpMessage):
        """Send message to client via stdout"""
        msg_json = json.dumps(message.to_dict())
        print(msg_json, flush=True)
        logger.debug(f"SENT: {msg_json}")

    async def handle_message(self, message: Dict) -> Optional[Dict]:
        """Handle incoming messages from client"""
        try:
            method = message.get("method")
            params = message.get("params", {})
            msg_id = message.get("id")

            logger.debug(f"RECEIVED: method={method}, id={msg_id}")

            # Handle initialize
            if method == "initialize":
                return McpMessage(
                    id=msg_id,
                    result={
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {"listChanged": False},
                            "resources": {"subscribe": False, "listChanged": False},
                            "prompts": {"listChanged": False},
                            "sampling": {},  # Server supports receiving sampling responses
                            "elicitation": {}  # Server supports receiving elicitation responses
                        },
                        "serverInfo": {
                            "name": "turbomcp-studio-test-server",
                            "version": "1.0.0"
                        }
                    }
                ).to_dict()

            # Handle initialized notification
            elif method == "notifications/initialized":
                self.initialized = True
                logger.info("Client initialized, starting test scenario...")

                # Start test scenario after initialization
                asyncio.create_task(self.run_test_scenario())
                return None

            # Handle sampling response (client completed sampling)
            elif method == "sampling/createMessage" or msg_id and "sampling" in str(msg_id):
                logger.info(f"SAMPLING RESPONSE received: {json.dumps(message, indent=2)}")

                if "result" in message:
                    result = message["result"]
                    self.test_results.append({
                        "test": "sampling",
                        "status": "success",
                        "response": result
                    })
                    logger.info(f"✅ Sampling test PASSED - received valid response")
                    logger.info(f"   Role: {result.get('role')}")
                    logger.info(f"   Model: {result.get('model')}")
                    logger.info(f"   Content: {result.get('content')}")
                elif "error" in message:
                    error = message["error"]
                    self.test_results.append({
                        "test": "sampling",
                        "status": "error",
                        "error": error
                    })
                    logger.warning(f"⚠️  Sampling test FAILED - received error: {error}")

                return None

            # Handle elicitation response
            elif method == "elicitation/input" or (msg_id and "elicitation" in str(msg_id)):
                logger.info(f"ELICITATION RESPONSE received: {json.dumps(message, indent=2)}")

                if "result" in message:
                    result = message["result"]
                    self.test_results.append({
                        "test": "elicitation",
                        "status": "success",
                        "response": result
                    })
                    logger.info(f"✅ Elicitation test PASSED - received valid response")
                    logger.info(f"   Action: {result.get('action')}")
                    if result.get('content'):
                        logger.info(f"   Content: {result.get('content')}")
                elif "error" in message:
                    error = message["error"]
                    self.test_results.append({
                        "test": "elicitation",
                        "status": "error",
                        "error": error
                    })
                    logger.warning(f"⚠️  Elicitation test FAILED - received error: {error}")

                return None

            # Unknown method
            else:
                return McpMessage(
                    id=msg_id,
                    error={
                        "code": -32601,
                        "message": f"Method not found: {method}"
                    }
                ).to_dict()

        except Exception as e:
            logger.error(f"Error handling message: {e}", exc_info=True)
            return McpMessage(
                id=message.get("id"),
                error={
                    "code": -32603,
                    "message": f"Internal error: {str(e)}"
                }
            ).to_dict()

    async def run_test_scenario(self):
        """Run the selected test scenario"""
        await asyncio.sleep(0.5)  # Give client time to settle

        scenario_func = self.scenarios.get(self.scenario)
        if scenario_func:
            logger.info(f"🧪 Starting test scenario: {self.scenario}")
            await scenario_func()
        else:
            logger.error(f"Unknown scenario: {self.scenario}")
            logger.info(f"Available scenarios: {list(self.scenarios.keys())}")

    async def test_simple_sampling(self):
        """Test basic sampling request"""
        logger.info("📝 Test: Simple Sampling Request")

        await self.send_message(McpMessage(
            id=self.next_id(),
            method="sampling/createMessage",
            params={
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "What is the capital of France?"
                        }
                    }
                ],
                "maxTokens": 100
            }
        ))

        logger.info("✅ Sent simple sampling request")

    async def test_complex_sampling(self):
        """Test complex multi-message sampling with system prompt"""
        logger.info("📝 Test: Complex Sampling Request")

        await self.send_message(McpMessage(
            id=self.next_id(),
            method="sampling/createMessage",
            params={
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "I'm planning a trip to Europe."
                        }
                    },
                    {
                        "role": "assistant",
                        "content": {
                            "type": "text",
                            "text": "That sounds exciting! Which countries are you considering?"
                        }
                    },
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "France and Italy. Can you suggest some cities?"
                        }
                    }
                ],
                "systemPrompt": "You are a helpful travel assistant with expertise in European destinations. Provide concise, practical advice.",
                "maxTokens": 200,
                "temperature": 0.7,
                "stopSequences": ["END", "STOP"]
            }
        ))

        logger.info("✅ Sent complex sampling request with conversation history")

    async def test_model_preferences(self):
        """Test model preference handling"""
        logger.info("📝 Test: Model Preferences")

        await self.send_message(McpMessage(
            id=self.next_id(),
            method="sampling/createMessage",
            params={
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "Explain quantum computing in simple terms."
                        }
                    }
                ],
                "systemPrompt": "You are an expert science educator.",
                "maxTokens": 300,
                "temperature": 0.5,
                "modelPreferences": {
                    "costPriority": 0.8,
                    "speedPriority": 0.5,
                    "intelligencePriority": 0.3,
                    "hints": [
                        "gpt-4o-mini",
                        "gpt-4o",
                        "claude-3-haiku",
                        "claude-3-5-sonnet"
                    ]
                }
            }
        ))

        logger.info("✅ Sent sampling request with model preferences")
        logger.info("   Cost priority: 0.8 (prefer cheaper models)")
        logger.info("   Speed priority: 0.5 (moderate speed)")
        logger.info("   Intelligence priority: 0.3 (lower complexity OK)")

    async def test_image_content(self):
        """Test image content in sampling request"""
        logger.info("📝 Test: Image Content")

        # Create a small test image (1x1 red pixel PNG)
        test_image_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg=="

        await self.send_message(McpMessage(
            id=self.next_id(),
            method="sampling/createMessage",
            params={
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "image",
                            "data": test_image_base64,
                            "mimeType": "image/png"
                        }
                    },
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "What do you see in this image?"
                        }
                    }
                ],
                "maxTokens": 150,
                "temperature": 0.7
            }
        ))

        logger.info("✅ Sent sampling request with image content")

    async def test_simple_elicitation(self):
        """Test simple text elicitation"""
        logger.info("📝 Test: Simple Elicitation")

        await self.send_message(McpMessage(
            id=self.next_id(),
            method="elicitation/input",
            params={
                "message": "Please enter your preferred model name for this session:",
                "schema": {
                    "type": "object",
                    "properties": {
                        "modelName": {
                            "type": "string",
                            "title": "Model Name",
                            "description": "The name of the LLM model you want to use",
                            "minLength": 3,
                            "maxLength": 50
                        }
                    },
                    "required": ["modelName"]
                }
            }
        ))

        logger.info("✅ Sent simple elicitation request")

    async def test_complex_schema(self):
        """Test complex schema with multiple field types"""
        logger.info("📝 Test: Complex Schema Validation")

        await self.send_message(McpMessage(
            id=self.next_id(),
            method="elicitation/input",
            params={
                "message": "Configure your LLM sampling preferences:",
                "schema": {
                    "type": "object",
                    "properties": {
                        "model": {
                            "type": "string",
                            "title": "Model",
                            "description": "Choose your preferred model",
                            "enum": ["gpt-4o", "gpt-4o-mini", "claude-3-5-sonnet", "claude-3-haiku"],
                            "default": "gpt-4o-mini"
                        },
                        "temperature": {
                            "type": "number",
                            "title": "Temperature",
                            "description": "Sampling temperature (0.0 = deterministic, 1.0 = creative)",
                            "minimum": 0.0,
                            "maximum": 1.0,
                            "default": 0.7
                        },
                        "maxTokens": {
                            "type": "integer",
                            "title": "Max Tokens",
                            "description": "Maximum tokens in response",
                            "minimum": 1,
                            "maximum": 4096,
                            "default": 1000
                        },
                        "enableStreaming": {
                            "type": "boolean",
                            "title": "Enable Streaming",
                            "description": "Stream responses as they are generated",
                            "default": False
                        },
                        "userEmail": {
                            "type": "string",
                            "title": "Email (optional)",
                            "description": "Your email for notifications",
                            "format": "email"
                        }
                    },
                    "required": ["model", "temperature", "maxTokens"]
                }
            }
        ))

        logger.info("✅ Sent elicitation request with complex schema")

    async def test_all_scenarios(self):
        """Run all test scenarios sequentially"""
        logger.info("🧪 Running ALL test scenarios...")

        scenarios = [
            ("simple", self.test_simple_sampling),
            ("complex", self.test_complex_sampling),
            ("preferences", self.test_model_preferences),
            ("image", self.test_image_content),
            ("elicitation", self.test_simple_elicitation),
            ("schema", self.test_complex_schema),
        ]

        for name, scenario_func in scenarios:
            logger.info(f"\n{'='*60}")
            logger.info(f"Running scenario: {name}")
            logger.info('='*60)
            await scenario_func()
            await asyncio.sleep(2)  # Wait between scenarios

        # Print summary after all tests
        await asyncio.sleep(5)
        self.print_test_summary()

    def print_test_summary(self):
        """Print test results summary"""
        logger.info("\n" + "="*60)
        logger.info("TEST SUMMARY")
        logger.info("="*60)

        if not self.test_results:
            logger.warning("No test results collected (tests may still be pending)")
            return

        passed = sum(1 for r in self.test_results if r["status"] == "success")
        failed = sum(1 for r in self.test_results if r["status"] == "error")

        logger.info(f"Total tests: {len(self.test_results)}")
        logger.info(f"Passed: {passed} ✅")
        logger.info(f"Failed: {failed} ❌")

        for i, result in enumerate(self.test_results, 1):
            status_icon = "✅" if result["status"] == "success" else "❌"
            logger.info(f"\n{i}. {result['test']}: {status_icon}")
            if result["status"] == "error":
                logger.info(f"   Error: {result['error']}")

        logger.info("="*60)

    async def run(self):
        """Main server loop"""
        logger.info("🚀 TurboMCP Studio Sampling & Elicitation Test Server starting...")
        logger.info(f"Scenario: {self.scenario}")
        logger.info("Waiting for client connection...\n")

        try:
            while True:
                # Read message from stdin
                line = await asyncio.get_event_loop().run_in_executor(
                    None, sys.stdin.readline
                )

                if not line:
                    break

                line = line.strip()
                if not line:
                    continue

                try:
                    message = json.loads(line)
                    response = await self.handle_message(message)

                    if response:
                        await self.send_message(McpMessage(**response))

                except json.JSONDecodeError as e:
                    logger.error(f"JSON decode error: {e}")
                    await self.send_message(McpMessage(
                        error={
                            "code": -32700,
                            "message": "Parse error"
                        }
                    ))

        except KeyboardInterrupt:
            logger.info("\n\n🛑 Server shutting down...")
            self.print_test_summary()

        except Exception as e:
            logger.error(f"Server error: {e}", exc_info=True)


def main():
    """Main entry point"""
    scenario = sys.argv[1] if len(sys.argv) > 1 else "simple"

    if scenario == "--help" or scenario == "-h":
        print(__doc__)
        sys.exit(0)

    server = SamplingElicitationTestServer(scenario)
    asyncio.run(server.run())


if __name__ == "__main__":
    main()
