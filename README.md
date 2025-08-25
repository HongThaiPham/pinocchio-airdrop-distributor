# Pinocchio Airdrop Distributor

A high-performance, gas-optimized Solana program for distributing airdrops using Merkle trees. Built with the [Pinocchio](https://github.com/anza-xyz/pinocchio) for maximum efficiency in a `no_std` environment.

## 🚀 Features

- **Merkle Tree-based Verification**: Efficient airdrop distribution using cryptographic proofs
- **Gas Optimized**: Built with Pinocchio framework for minimal compute usage
- **No-std Environment**: Zero heap allocations, stack-only operations
- **Secure**: Cryptographically secure claim verification

### Core Instructions

1. **Initialize Airdrop** - Create a new airdrop campaign with merkle root
2. **Claim Airdrop** - Allow eligible users to claim their tokens
3. **Update Merkle Root** - Admin function to update the merkle tree

### Hash Function

Uses Keccak256 via `solana-nostd-keccak` for compatibility and performance.

## 🧪 Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_hash_functions -- --nocapture

# Run merkle tree tests
cargo test test_create_merkle_root_and_proof -- --nocapture
```

### Test Coverage

- ✅ Hash function correctness verification
- ✅ Merkle tree construction and verification
- ✅ Merkle proof generation and validation
- ✅ Airdrop initialization
- ✅ Claim instruction data parsing
- ✅ End-to-end claim workflow

## 📦 Dependencies

### Runtime Dependencies

- `pinocchio` - Core Solana program framework
- `pinocchio-pubkey` - Pubkey utilities
- `pinocchio-system` - System program interactions
- `solana-nostd-keccak` - Keccak hash function for no_std

### Development Dependencies

- `mollusk-svm` - Solana program testing framework
- `solana-sdk` - Solana development kit
- `pinocchio-log` - Logging utilities

## 🔧 Building

### Prerequisites

- Rust 1.88.0
- Solana CLI 2.1.0
- Pinocchio 0.9.0

### Build Commands

```bash
# Build the program
cargo build-sbf

# Build for release
cargo build-sbf --release

# Deploy to devnet
solana program deploy target/deploy/pinocchio_airdrop_distributor.so
```

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Pinocchio](https://github.com/anza-xyz/pinocchio) - High-performance Solana program framework
- [Solana Labs](https://solana.com/) - Blockchain infrastructure
- [Mollusk SVM](https://github.com/anza-xyz/mollusk) - Testing framework

---

Built with ❤️ for the Solana ecosystem
