#!/bin/bash
# TTGTiSO-Desk API Test Suite

API_URL="http://127.0.0.1:3001/api"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🧪 TTGTiSO-Desk API Test Suite"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Test 1: Health Check
echo "1️⃣  Health Check"
response=$(curl -s $API_URL/health)
if [ "$response" = "OK" ]; then
    echo "   ✅ API Server is running"
else
    echo "   ❌ API Server is not responding"
    exit 1
fi
echo ""

# Test 2: Get Hosts
echo "2️⃣  Get Hosts"
hosts_count=$(curl -s $API_URL/hosts | jq 'length')
echo "   📊 Found $hosts_count hosts"
echo "   Sample:"
curl -s $API_URL/hosts | jq '.[0] | {name, ip, status}'
echo ""

# Test 3: Get Active Sessions
echo "3️⃣  Get Active Sessions"
sessions_count=$(curl -s $API_URL/sessions/active | jq 'length')
echo "   📊 Found $sessions_count active sessions"
echo "   Sample:"
curl -s $API_URL/sessions/active | jq '.[0] | {username, display_id, cpu_usage}'
echo ""

# Test 4: Get Logs
echo "4️⃣  Get Logs"
logs_count=$(curl -s $API_URL/logs | jq 'length')
echo "   📊 Found $logs_count log entries"
echo "   Sample (last 2):"
curl -s $API_URL/logs | jq '.[-2:] | .[] | {timestamp, level, message}'
echo ""

# Test 5: Get Settings
echo "5️⃣  Get Settings"
curl -s $API_URL/settings | jq '.'
echo ""

# Test 6: Login
echo "6️⃣  Login Test"
login_response=$(curl -s -X POST $API_URL/auth/login \
  -H "Content-Type: application/json" \
  -d '{"host":"192.168.1.100","port":22,"username":"test_user","password":"test"}')
echo "   Response:"
echo "$login_response" | jq '.'
echo ""

# Test 7: Update Settings
echo "7️⃣  Update Settings Test"
update_response=$(curl -s -X POST $API_URL/settings \
  -H "Content-Type: application/json" \
  -d '{"quality":"high","encoder":"vaapi","fps":60,"audio":true}')
echo "   Response:"
echo "$update_response" | jq '.'
echo ""

# Test 8: Terminate Session (will fail if session doesn't exist, that's ok)
echo "8️⃣  Terminate Session Test (s1)"
terminate_response=$(curl -s -X POST $API_URL/sessions/s1/terminate)
if [ $? -eq 0 ]; then
    echo "   Response:"
    echo "$terminate_response" | jq '.'
else
    echo "   ⚠️  Session not found (expected if already terminated)"
fi
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ API Test Suite Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
