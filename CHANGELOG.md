## [Release 0.8.0-develop.1](https://github.com/Arrow-air/svc-cargo/releases/tag/v0.8.0-develop.1)

### 🛠 Maintenance

-  **ci:** .make/env.mk - provisioned by terraform ([`afccef9`](https://github.com/Arrow-air/svc-cargo/commit/afccef9a345a0239b8b8cad7f607b3da235a8ec5))
-  **ci:** .env.base - provisioned by terraform ([`2332fda`](https://github.com/Arrow-air/svc-cargo/commit/2332fda7488bad6b6a4152b973572094c4282f61))
-  **ci:** .github/workflows/nightly.yml - provisioned by terraform ([`9f55cb0`](https://github.com/Arrow-air/svc-cargo/commit/9f55cb0f1d6297e82c78762ee0b81147737e7420))
-  **ci:** license - provisioned by terraform ([`7731939`](https://github.com/Arrow-air/svc-cargo/commit/77319391c503def69a1cc484e8a328d3c2e78c76))
-  **ci:** .editorconfig - provisioned by terraform ([`4763e33`](https://github.com/Arrow-air/svc-cargo/commit/4763e336c0154730e494172876b0de5760afaa2b))
-  **ci:** .github/workflows/release.yml - provisioned by terraform ([`48b8b50`](https://github.com/Arrow-air/svc-cargo/commit/48b8b507ad8899d50048f170a3d0641da8f35ac0))

### 📚 Documentation

-  **readme:** add license notice and additional info (#24) ([`7209bab`](https://github.com/Arrow-air/svc-cargo/commit/7209bab6154b077fb3b19ca7a72e03bc74f697fd))

## [Release 0.8.0-develop.0](https://github.com/Arrow-air/svc-cargo/releases/tag/v0.8.0-develop.0)

### ✨ Features

-  **rest:** add REST interfaces ([`f4b57de`](https://github.com/Arrow-air/svc-cargo/commit/f4b57de43ac59cc53ba6eea73b392b759b18acd6))
-  **grpc:** add grpc server with health check and example (#3) ([`f9fa48f`](https://github.com/Arrow-air/svc-cargo/commit/f9fa48f8b3860ff7dab3fa1bd074b43adae59f71))
-  **grpc:** add grpc connection to svc-storage ([`0679a64`](https://github.com/Arrow-air/svc-cargo/commit/0679a64c424641f1e07cfec9c5c7fa87754fde04))
-  **rest:** update REST API to accept arrival time window for flight query ([`885a200`](https://github.com/Arrow-air/svc-cargo/commit/885a200f54b0886317d4498bdfbe4e1e13110b5c))
-  **grpc:** add svc-pricing connections ([`fc95ae5`](https://github.com/Arrow-air/svc-cargo/commit/fc95ae55cca42aad823fa007fdc63d4bed5c812d))
-  **rest:** add log statements, rest code to rest_api.rs (#15) ([`d30780b`](https://github.com/Arrow-air/svc-cargo/commit/d30780b6266378ac7f90b130c19f446d64688fad))

### 🐛 Fixes

-  **grpc:** use env variables for grpc clients ([`b810671`](https://github.com/Arrow-air/svc-cargo/commit/b8106714d462f6fadf39a9bc58606245b6279927))
-  **cargo:** add vendored-openssl feature ([`489b109`](https://github.com/Arrow-air/svc-cargo/commit/489b10931146c6fbce897865a13530bc5d0b92f9))
- use base_pricing instead of customer_cost ([`877d319`](https://github.com/Arrow-air/svc-cargo/commit/877d31912c435b1c244e0df36f8337af682ca277))
- address r1 review comments ([`9e75654`](https://github.com/Arrow-air/svc-cargo/commit/9e75654e7688cb94b6abb72db5c55e45cbf037e7))

### 🛠 Maintenance

-  **log:** switch from env_logger to log4rs (#20) ([`16de514`](https://github.com/Arrow-air/svc-cargo/commit/16de514c5f76a561c7b0019330a7460835ecc89a))
-  **toml:** remove minor versions (#21) ([`d07b648`](https://github.com/Arrow-air/svc-cargo/commit/d07b64832027ca922e7f7740fea480cef2010cac))
- clear changelog ([`6aa9bd8`](https://github.com/Arrow-air/svc-cargo/commit/6aa9bd88354956b23b822f5a5d6cac56e644b069))

### 📚 Documentation

-  **sdd:** add SDD ([`99d12b6`](https://github.com/Arrow-air/svc-cargo/commit/99d12b680eee7646f560527c8255ade0eb6a3899))

