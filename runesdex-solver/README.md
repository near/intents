# RunesDex Solver for NEAR Intents

This module provides integration between RunesDex and NEAR Intents, enabling seamless token swaps across these platforms.

## Features

- Token swap integration between RunesDex and NEAR blockchain
- Efficient price discovery and execution via the NEAR Intents protocol
- Full support for RunesDex API
- Implementation of the solver pattern for optimal execution

## Overview

This solver works by:

1. Monitoring for swap intents on the NEAR blockchain
2. Calculating optimal swap routes and prices using RunesDex API
3. Executing the swaps and reporting back to the NEAR Intents protocol

## Usage

The solver can be deployed as a service that continuously monitors for new intents and provides swap solutions:

```sh
cargo run --release
```

## Configuration

Configuration is handled via environment variables:

- `RUNESDEX_API_KEY`: Your API key for RunesDex
- `NEAR_ACCOUNT_ID`: The account ID to use for NEAR transactions
- `NEAR_PRIVATE_KEY`: The private key for signing transactions
- `SOLVER_BUS_URL`: URL of the solver bus to connect to

## License

MIT 