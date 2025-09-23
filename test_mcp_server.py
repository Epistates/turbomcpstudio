#!/usr/bin/env python3
"""
Simple MCP test server for validating MCP Studio functionality.
Implements basic tools and resources for testing.
"""

import asyncio
import json
import sys
import logging
from typing import Any, Dict, List, Optional

# Basic MCP protocol messages
class McpMessage:
    def __init__(self, jsonrpc: str = "2.0", id: Optional[str] = None, method: Optional[str] = None, params: Optional[Dict] = None, result: Optional[Any] = None, error: Optional[Dict] = None):
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

class McpTestServer:
    def __init__(self):
        self.tools = {
            "echo": {
                "name": "echo",
                "description": "Echo back the input text",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "Text to echo back"
                        }
                    },
                    "required": ["text"]
                }
            },
            "calculate": {
                "name": "calculate",
                "description": "Perform basic mathematical calculations",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string",
                            "description": "Mathematical expression to evaluate (e.g., '2 + 2')"
                        }
                    },
                    "required": ["expression"]
                }
            },
            "get_time": {
                "name": "get_time",
                "description": "Get the current time",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        }

    async def handle_message(self, message: Dict) -> Dict:
        """Handle incoming MCP messages"""
        try:
            method = message.get("method")
            params = message.get("params", {})
            msg_id = message.get("id")

            if method == "initialize":
                return McpMessage(
                    id=msg_id,
                    result={
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {
                                "listChanged": True
                            },
                            "resources": {
                                "subscribe": True,
                                "listChanged": True
                            },
                            "prompts": {
                                "listChanged": True
                            }
                        },
                        "serverInfo": {
                            "name": "test-mcp-server",
                            "version": "1.0.0"
                        }
                    }
                ).to_dict()

            elif method == "notifications/initialized":
                # No response needed for notification
                return None

            elif method == "tools/list":
                return McpMessage(
                    id=msg_id,
                    result={
                        "tools": list(self.tools.values())
                    }
                ).to_dict()

            elif method == "tools/call":
                tool_name = params.get("name")
                arguments = params.get("arguments", {})

                if tool_name == "echo":
                    text = arguments.get("text", "")
                    return McpMessage(
                        id=msg_id,
                        result={
                            "content": [
                                {
                                    "type": "text",
                                    "text": f"Echo: {text}"
                                }
                            ]
                        }
                    ).to_dict()

                elif tool_name == "calculate":
                    expression = arguments.get("expression", "")
                    try:
                        # Simple evaluation (DANGEROUS in production - only for testing)
                        result = eval(expression.replace("^", "**"))
                        return McpMessage(
                            id=msg_id,
                            result={
                                "content": [
                                    {
                                        "type": "text",
                                        "text": f"Result: {result}"
                                    }
                                ]
                            }
                        ).to_dict()
                    except Exception as e:
                        return McpMessage(
                            id=msg_id,
                            error={
                                "code": -32000,
                                "message": f"Calculation error: {str(e)}"
                            }
                        ).to_dict()

                elif tool_name == "get_time":
                    import datetime
                    current_time = datetime.datetime.now().isoformat()
                    return McpMessage(
                        id=msg_id,
                        result={
                            "content": [
                                {
                                    "type": "text",
                                    "text": f"Current time: {current_time}"
                                }
                            ]
                        }
                    ).to_dict()

                else:
                    return McpMessage(
                        id=msg_id,
                        error={
                            "code": -32601,
                            "message": f"Tool not found: {tool_name}"
                        }
                    ).to_dict()

            else:
                return McpMessage(
                    id=msg_id,
                    error={
                        "code": -32601,
                        "message": f"Method not found: {method}"
                    }
                ).to_dict()

        except Exception as e:
            return McpMessage(
                id=message.get("id"),
                error={
                    "code": -32603,
                    "message": f"Internal error: {str(e)}"
                }
            ).to_dict()

    async def run(self):
        """Run the MCP server using stdio"""
        logging.basicConfig(level=logging.DEBUG, filename='/tmp/mcp_test_server.log')
        logging.info("MCP Test Server starting...")

        try:
            while True:
                # Read message from stdin
                line = await asyncio.get_event_loop().run_in_executor(None, sys.stdin.readline)
                if not line:
                    break

                line = line.strip()
                if not line:
                    continue

                logging.info(f"Received: {line}")

                try:
                    message = json.loads(line)
                    response = await self.handle_message(message)

                    if response:
                        response_json = json.dumps(response)
                        print(response_json, flush=True)
                        logging.info(f"Sent: {response_json}")

                except json.JSONDecodeError as e:
                    logging.error(f"JSON decode error: {e}")
                    error_response = McpMessage(
                        error={
                            "code": -32700,
                            "message": "Parse error"
                        }
                    ).to_dict()
                    print(json.dumps(error_response), flush=True)

        except KeyboardInterrupt:
            logging.info("Server shutting down...")
        except Exception as e:
            logging.error(f"Server error: {e}")

if __name__ == "__main__":
    asyncio.run(McpTestServer().run())