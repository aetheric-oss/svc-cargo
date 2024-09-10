## [Release 0.11.0](https://github.com/aetheric-oss/svc-cargo/releases/tag/v0.11.0)

### ✨ Features

- add get parcel scans endpoint ([`fc658b4`](https://github.com/aetheric-oss/svc-cargo/commit/fc658b4f7405771a9363126c05d3293c17de83b1))
- add redis connection, update scheduler deps ([`66d51ce`](https://github.com/aetheric-oss/svc-cargo/commit/66d51cef907c6f93527d5e5778f6dcaac6c21a84))
- add call to svc-contact for confirmation email ([`4112aa2`](https://github.com/aetheric-oss/svc-cargo/commit/4112aa206cc72c1bb5f65b845788bc75f0b4454f))
- remove time arrival window ([`cbb39c8`](https://github.com/aetheric-oss/svc-cargo/commit/cbb39c8e41e423ad93818d56774304d77ed3d75c))
- update arrowair nocodb references ([`55a35c7`](https://github.com/aetheric-oss/svc-cargo/commit/55a35c7a4434a201e453a3ca6007b82533e78fd4))
- final cleanup ([`8253895`](https://github.com/aetheric-oss/svc-cargo/commit/825389564d988b71af088eae245a7794488badf7))

### 🐛 Fixes

- don't expose flight plan storage data, fix docker file ([`a59721f`](https://github.com/aetheric-oss/svc-cargo/commit/a59721f698b707f40447e0f03f43200b2beebe85))
- get vertiport IDs, not provided by default ([`ed79e11`](https://github.com/aetheric-oss/svc-cargo/commit/ed79e11c10d769367de3efd9cdc66cc1ce6e2f52))
- add unwrap error log to config try ([`1cefa7e`](https://github.com/aetheric-oss/svc-cargo/commit/1cefa7e5f7c726439607606e65068d293ddba295))
- rest API schemes ([`976fd88`](https://github.com/aetheric-oss/svc-cargo/commit/976fd88eaa614701659b50ca74b91e6b104f8cd8))
- update no_coverage releases ([`3428f4d`](https://github.com/aetheric-oss/svc-cargo/commit/3428f4d9d55f1a938761da4304067979d2ccde26))
- authors in docs and Cargo file ([`f6c45e8`](https://github.com/aetheric-oss/svc-cargo/commit/f6c45e89b30b2f1c15298003ec865cc4ff61004f))

### 🛠 Maintenance

- terraform provisioned file changes ([`2851eb3`](https://github.com/aetheric-oss/svc-cargo/commit/2851eb34e5455fecb5e52143a829cc2a0437ea2e))
- reviewer comments ([`335a0d6`](https://github.com/aetheric-oss/svc-cargo/commit/335a0d68b6e69387ddd8810909ecb1e648f71813))
- bring repo in line with template ([`f0d0fae`](https://github.com/aetheric-oss/svc-cargo/commit/f0d0faef69f2d42f523cef159fc61ec0f5c4347b))
- tofu provisioned file changes ([`272c098`](https://github.com/aetheric-oss/svc-cargo/commit/272c098f2d4a439baebdc3d21d682d218d410831))
- update duration to try_ method ([`b827f06`](https://github.com/aetheric-oss/svc-cargo/commit/b827f062841d339c54e634107721ebffe3f47dfb))
- update dependencies ([`4d5dce5`](https://github.com/aetheric-oss/svc-cargo/commit/4d5dce581bcb78edbfad062dfe185a795e5f06d5))
- reviewer comments ([`551d14a`](https://github.com/aetheric-oss/svc-cargo/commit/551d14a5a036bd3d3df56611653ad5d823f9106c))
- fix logging messages ([`5a24470`](https://github.com/aetheric-oss/svc-cargo/commit/5a24470f0f547d9799df210cbb9db12c265d8707))
- cleanup and update unit tests ([`1b05ff7`](https://github.com/aetheric-oss/svc-cargo/commit/1b05ff77c6e1c40fc0a1fb65399a09c5da00e519))
- remove function prepend, automatic now with lib-common ([`d68eabd`](https://github.com/aetheric-oss/svc-cargo/commit/d68eabdeb4362242fa405c281192c543716dba6c))
- reviewer comments 1 ([`41a7901`](https://github.com/aetheric-oss/svc-cargo/commit/41a7901b5736914cff12cc50c1defb0864305bc6))

## [Release 0.10.0](https://github.com/Arrow-air/svc-cargo/releases/tag/v0.10.0)

### ✨ Features

- add parcel scan API ([`b66df66`](https://github.com/Arrow-air/svc-cargo/commit/b66df66dcc65f77bc52d3d51359f5d50dea468d3))
- connect scan api to storage ([`8a9e48b`](https://github.com/Arrow-air/svc-cargo/commit/8a9e48bcc52ae71b9c96bede19fc0e73e13b7bf0))
- add get flight plans API ([`c19c4f3`](https://github.com/Arrow-air/svc-cargo/commit/c19c4f33a485056bcec043c02644f8fb41fa223b))
- add rate and request buffer limiting ([`995257a`](https://github.com/Arrow-air/svc-cargo/commit/995257a078ffc2b62dc28822e228d76d966b3e8d))

### 🛠 Maintenance

- terraform provisioned file changes ([`734b4c6`](https://github.com/Arrow-air/svc-cargo/commit/734b4c6fadad167ca45a39d5cc7a51f1fd52be18))
- address reviewer comments ([`4da2da2`](https://github.com/Arrow-air/svc-cargo/commit/4da2da26622e614f9ebc3300e0111f02e4d1e277))
- refactor ([`cc71dce`](https://github.com/Arrow-air/svc-cargo/commit/cc71dce2713d6005f43b9b1e1c2f2d7025b89c13))
- r3 final cleanup ([`7e00963`](https://github.com/Arrow-air/svc-cargo/commit/7e009638a9565c42c2c3c01a584123ee7e9c1a76))
- update dependencies ([`86c395a`](https://github.com/Arrow-air/svc-cargo/commit/86c395a56feecc908866eac5871acbb188425ce0))
- r3 final cleanup ([`32c7de3`](https://github.com/Arrow-air/svc-cargo/commit/32c7de3ee6380b8a781c200f52355a13a0e57d03))

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
