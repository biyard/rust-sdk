# ICP Canister

This section covers the Internet Computer Protocol (ICP) canister integration within the Biyard ecosystem. The implementation provides seamless interaction between frontend applications and ICP smart contracts (canisters).

## Overview

The ICP integration enables:
- Secure authentication using Internet Identity.
- NFT management on the ICP blockchain.
- Cross-chain bridging functionality.
- Query and update operations with canisters.

## Key Features

- **Canister Interaction**: Query and update canister state through agent-based communication.
- **NFT Management**: List, bridge, and manage NFTs across chains.
- **Identity Integration**: Unified wallet system supporting both EVM and ICP identities.
- **Error Handling**: Comprehensive error management for blockchain operations.

## Getting Started

To begin working with ICP canisters in your project:

1. Configure your canister details in `config.rs`.
2. Initialize the `IcpCanister` service in your application.
3. Use the provided methods to interact with your canister.