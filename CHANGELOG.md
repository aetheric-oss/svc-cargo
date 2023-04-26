## [Release 0.9.0](https://github.com/Arrow-air/svc-cargo/releases/tag/v0.9.0)

### ✨ Features

- r2 api updates, autogen openapi spec ([`e985873`](https://github.com/Arrow-air/svc-cargo/commit/e985873f3af02398875a6d5e1a89063d16d1b9d9))
- add health endpoint ([`4b12ee0`](https://github.com/Arrow-air/svc-cargo/commit/4b12ee0d2b4dbf6b8c99d860627ed66944ee5faf))

### 🐛 Fixes

-  **rest:** add tag to utoipa paths ([`c1b8a4d`](https://github.com/Arrow-air/svc-cargo/commit/c1b8a4df2a95493ff5df745c1942d9ea4133a0df))

### 🛠 Maintenance

- update release files ([`a30f598`](https://github.com/Arrow-air/svc-cargo/commit/a30f5984bcaad3cb3143ecece7f588b6bcc1cec4))
- terraform provisioned file changes ([`1909502`](https://github.com/Arrow-air/svc-cargo/commit/1909502714d2dd03ccf6a1272f82b7a933947e39))
-  **cargo:** use release tags for arrow dependencies ([`27f4332`](https://github.com/Arrow-air/svc-cargo/commit/27f4332ef3d10aa6e25fd5f02b51051c33bf1755))
- add support for multi-leg itineraries ([`6afbd41`](https://github.com/Arrow-air/svc-cargo/commit/6afbd419ebf5d85b9d90720d7633231685e37dc4))
- update to use itineraries ([`850a0de`](https://github.com/Arrow-air/svc-cargo/commit/850a0de56bba4d255b285698ea2871e043ac762f))
- module refactor ([`98475fb`](https://github.com/Arrow-air/svc-cargo/commit/98475fb0b2d24836ac9b762dc8af2496a3813132))
- cleanup ([`5e780c4`](https://github.com/Arrow-air/svc-cargo/commit/5e780c4a032da15d13b1aa42a859deaeb56a5c42))
- address reviewer comments ([`cbe42d0`](https://github.com/Arrow-air/svc-cargo/commit/cbe42d09bf0f5c1de77460d89a34f66cff254cae))

### 📚 Documentation

-  **readme:** add license notice and additional info (#24) ([`0b9c2f2`](https://github.com/Arrow-air/svc-cargo/commit/0b9c2f244318f7e82d75581ec27df83af6f85e8e))
-  **conops:** add conops (#26) ([`551c04e`](https://github.com/Arrow-air/svc-cargo/commit/551c04e348da07b0c8d9570f3bf240ef9ffb50d1))

## [Release 0.2.0](https://github.com/Arrow-air/svc-cargo/releases/tag/v0.2.0)

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
