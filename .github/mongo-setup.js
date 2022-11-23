use('analog_db');
db.createCollection('contracts')
db.createCollection('events')

db.contracts.insertOne({
    chain_endpoint: "ws://127.0.0.1:8545/",
    contract_address: "0x5FbDB2315678afecb367f032d93F642f64180aa3",
    event_type: "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
})
