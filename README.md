[![Continuous Integration](https://github.com/lthiery/hnt-explorer/actions/workflows/rust.yml/badge.svg)](https://github.com/lthiery/hnt-explorer/actions/workflows/rust.yml)

# hnt-explorer

This application extracts data from helium-programs via Solana RPC and serves it via HTTP. There are CLI commands
meant to run and test the data extraction logic.

## Endpoints 

GET `/v1/delegated_stakes`
Params: `limit`, `start`, `timestamp`

Provides list of delegated stakes. When no timestamp is provided, the latest pulled data is used, including timestamp.
Use the timestamp to maintain index on the same batch of data and start and limit to fetch more positions.

When no limit is provided, default of 500 items is used. limit is capped also at 500.

When no start is provided, default of 0 is used.

GET `/v1/delegated_stakes/csv`

Serves most recent list of delegated stakes as a CSV file.

GET `/v1/delegated_stakes/info`

GET `/v1/epoch/info`

## Environmental variables

* `SOL_RPC_ENDPOINT` - Solana RPC URL (defaults to `https://api.mainnet-beta.solana.com`)
* `PORT` - Port to listen on (defaults to `3000`)

## Pushing to heroku 

Build the container:
```
heroku container:push web 
```

Release it:
```
heroku container:release web 
```

Check logs:
```
heroku logs --tail 
```
