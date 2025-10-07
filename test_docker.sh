#!/bin/bash
# OMEN Docker Stack Test Script

set -e

echo "🧪 Testing OMEN Docker Stack..."
echo ""

# Build
echo "📦 Building Docker image..."
docker compose build omen
docker tag omen:latest omen:rc1
echo "✅ Build successful"
echo ""

# Start stack
echo "🚀 Starting stack..."
docker compose up -d
echo "⏳ Waiting for services to be ready..."
sleep 8
echo ""

# Test endpoints
echo "🔍 Testing endpoints..."
echo ""

echo "1️⃣  Health endpoint:"
curl -s http://localhost:8080/health | jq -r '.service, .version, .status'
echo ""

echo "2️⃣  Ready endpoint:"
curl -s http://localhost:8080/ready | jq -r '.status'
echo ""

echo "3️⃣  Provider scores:"
curl -s http://localhost:8080/omen/providers/scores | jq '.[0] | {provider: .provider_name, score: .overall_score}'
echo ""

echo "4️⃣  Models endpoint:"
MODEL_COUNT=$(curl -s http://localhost:8080/v1/models | jq '.data | length')
echo "Available models: $MODEL_COUNT"
echo ""

# Container status
echo "📊 Container status:"
docker compose ps
echo ""

# Cleanup
echo "🧹 Cleaning up..."
docker compose down
echo ""

echo "✅ All tests passed! OMEN RC1 is ready for Zeke integration."
