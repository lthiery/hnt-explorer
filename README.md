[![Continuous Integration](https://github.com/lthiery/hnt-explorer/actions/workflows/rust.yml/badge.svg)](https://github.com/lthiery/hnt-explorer/actions/workflows/rust.yml)

# hnt-explorer

This application extracts data from helium-programs via Solana RPC and serves it via HTTP. There are CLI commands
meant to run and test the data extraction logic.

## Endpoints 

GET `/v1/delegated_stakes`
Params: `limit`, `start`, `timestamp`

Provides list of delegated stakes. When no timestamp is provided, the latest pulled data is used, including timestamp.
Use the timestamp to maintained index on the same batch of data.

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
