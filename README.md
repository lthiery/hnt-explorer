[![Continuous Integration](https://github.com/lthiery/hnt-explorer/actions/workflows/rust.yml/badge.svg)](https://github.com/lthiery/hnt-explorer/actions/workflows/rust.yml)

# hnt-explorer

This application extracts data from helium-programs via Solana RPC and serves it via HTTP. There are CLI commands meant
to run and test the data extraction logic.

## Building and running

You only need to do this if you want to run the server locally and/or make changes. Otherwise, skip to the next section.

`cargo run -- server` starts the server. By default, it will hit the public Solana RPC **so this will probably error**. 
Use the environmental variable `SOL_RPC_ENDPOINT` to set a different RPC endpoint (checkout [Helius](https://www.helius.xyz/)
for example).

## Public endpoint

I am currently hosting a public endpoint here: https://hnt-explorer.herokuapp.com. 
Prepend the paths below with the endpoint.

## Endpoints 

GET `/v1/accounts/{account}`
Provides balances of HNT, MOBILE, and IOT. This endpoint is aware of positions and delegated stakes and will provide
"locked" and "pending" amounts for balances. This endpoint is not aware of pending hotspot rewards (nor does it list
hotspot NFTs).

GET [`/v1/positions`](https://hnt-explorer.herokuapp.com/v1/positions)

Params: `limit`, `start`, `timestamp`

Provides list of all positions. When no timestamp is provided, the latest pulled data is used, including timestamp.
Data is pulled every 5 minutes. Use the timestamp to maintain index on the same batch of data and start and limit to
fetch more positions.

When no limit is provided, default of 500 items is used. limit is capped also at 500.

When no start is provided, default of 0 is used.

If using more than one parameter at a time, all parameters must be encapsulated in a string. For example:

```
https://hnt-explorer.herokuapp.com/v1/positions?"timestamp=1682720623?start=500"
```

GET `/v1/positions/{position}`

Provides data of a specific position, including most recently derived veHNT (at most 5 minutes old) and pending rewards.

GET [`/v1/positions/csv`](https://hnt-explorer.herokuapp.com/v1/positions/csv)

Serves most recent list of all positions as a CSV file.

GET [`/v1/positions/info`](https://hnt-explorer.herokuapp.com/v1/delegated_stakes/info)

GET [`/v1/epoch/info`](https://hnt-explorer.herokuapp.com/v1/epoch/info)

## Legacy Endpoints

Warning: these will be deprecated soon.

GET [`/v1/delegated_stakes`](https://hnt-explorer.herokuapp.com/v1/delegated_stakes)

Params: `limit`, `start`, `timestamp`

Provides list of delegated stakes. When no timestamp is provided, the latest pulled data is used, including timestamp.
Data is pulled every 5 minutes. Use the timestamp to maintain index on the same batch of data and start and limit to
fetch more positions.

When no limit is provided, default of 500 items is used. limit is capped also at 500.

When no start is provided, default of 0 is used.

If using more than one parameter at a time, all parameters must be encapsulated in a string. For example:

```
https://hnt-explorer.herokuapp.com/v1/delegated_stakes?"timestamp=1682720623?start=500"
```

GET [`/v1/delegated_stakes/csv`](https://hnt-explorer.herokuapp.com/v1/delegated_stakes/csv)

Serves most recent list of delegated stakes as a CSV file.

GET [`/v1/delegated_stakes/info`](https://hnt-explorer.herokuapp.com/v1/delegated_stakes/info)

## Environmental variables

* `SOL_RPC_ENDPOINT` - Solana RPC URL (defaults to `https://api.mainnet-beta.solana.com`)
* `PORT` - Port to listen on (defaults to `3000`)

## Pushing to heroku

You can host this program on Heroku easily by creating an app and pushing the container to it. Just change the app name
below from `hnt-explorer` to whatever your app name is.

Build the container:
```
heroku container:push web -a hnt-explorer
```

Release it:
```
heroku container:release web -a hnt-explorer 
```

Check logs:
```
heroku logs --tail -a hnt-explorer
```
