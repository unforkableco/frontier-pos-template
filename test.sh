curl -X POST http://127.0.0.1:9944 \
  -H "Content-Type: application/json" \
  --data '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "eth_call",
    "params": [{
      "from": "0xc723b594543a026ee7663dbb092c0e0b792ef13c",
      "to": "0x0000000000000000000000000000000000000500",
      "data": "0xa9059cbb000000000000000000000000f24ff3a9cf04c71dbc94d0b566f7a27b94566cac0000000000000000000000000000000000000000000000008ac7230489e80000",
      "value": "0x8ac7230489e80000"
    }, "latest"]
  }'