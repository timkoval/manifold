#!/bin/bash

# Test MCP server with JSON-RPC 2.0 requests over stdio

echo "Testing MCP Server (JSON-RPC 2.0 over stdio)"
echo "============================================="
echo

# Test 1: Initialize
echo "Test 1: Initialize"
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | ./target/debug/manifold serve
echo
echo

# Test 2: List tools  
echo "Test 2: List tools"
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | ./target/debug/manifold serve
echo
echo

# Test 3: Query manifold
echo "Test 3: Query existing specs"
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"query_manifold","arguments":{}}}' | ./target/debug/manifold serve
echo
echo

echo "Tests complete"
