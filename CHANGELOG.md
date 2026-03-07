# Changelog

## [Unreleased]

## [0.3.3](https://github.com/bobchevalieragility/arc-cli/compare/v0.3.2...v0.3.3)

### 🐛 Bug Fixes


- Force bash shell in CI - ([d49850f](https://github.com/bobchevalieragility/arc-cli/commit/d49850f7d1f2cb76e71573b6dc68702465a678f6))


## [0.3.2](https://github.com/bobchevalieragility/arc-cli/compare/v0.3.1...v0.3.2)

### ⛰️ Features


- Add windows release - ([0da1f78](https://github.com/bobchevalieragility/arc-cli/commit/0da1f783b74e969bfc6f7935bcd91fca35e8e11f))


## [0.3.1](https://github.com/bobchevalieragility/arc-cli/compare/v0.3.0...v0.3.1)

### 🐛 Bug Fixes


- Extract Argo workflow-worker version from Sensor resource - ([deb78f9](https://github.com/bobchevalieragility/arc-cli/commit/deb78f9673998d1f4b59a0adbda370331a2a43a8))
- Fix duplicate Argo app status rows - ([bfccfb1](https://github.com/bobchevalieragility/arc-cli/commit/bfccfb1b2eed1cd8fe96182d798583998a7b391d))

### 🚜 Refactor


- Create Get and Set subcommands of Logging - ([2019e5f](https://github.com/bobchevalieragility/arc-cli/commit/2019e5f9b00f88d2c4dee8c5bc9d22b2d013720a))
- Make vault and aws secrets mgr commands sub-commands - ([ffee175](https://github.com/bobchevalieragility/arc-cli/commit/ffee1755c10e667cc60fb482589ccf3afcaa24d8))


## [0.3.0](https://github.com/bobchevalieragility/arc-cli/compare/v0.2.6...v0.3.0)

### ⛰️ Features


- Monitor ArgoCD application statuses - ([6b870df](https://github.com/bobchevalieragility/arc-cli/commit/6b870dfc04dcc114c474c53fa9e550b15e8357c0))


## [0.2.6](https://github.com/bobchevalieragility/arc-cli/compare/v0.2.5...v0.2.6)

### ⛰️ Features


- Add --raw-output arg to allow arc to be called from within a script - ([883b1ff](https://github.com/bobchevalieragility/arc-cli/commit/883b1ff99b2ba9c7aecf583468130cd39c009788))


## [0.2.5](https://github.com/bobchevalieragility/arc-cli/compare/v0.2.4...v0.2.5)

### ⛰️ Features


- Make --aws-profile and --kube-context global params - ([c9e857b](https://github.com/bobchevalieragility/arc-cli/commit/c9e857b63398cb8e62282b4c0aa41f32b4b6d7f2))


## [0.2.4](https://github.com/bobchevalieragility/arc-cli/compare/v0.2.3...v0.2.4)

### ⛰️ Features


- Allow AWS profile and K8 context to be explicitly specified - ([4a6ea0a](https://github.com/bobchevalieragility/arc-cli/commit/4a6ea0aecd4ebf17e45b189f866a621ea02e0294))
- Allow port-forward groups - ([a35b0d7](https://github.com/bobchevalieragility/arc-cli/commit/a35b0d794941486eee0949c8b362f930b779458b))

### 📚 Documentation


- Include link to task-chaining writeup in README - ([caaccc5](https://github.com/bobchevalieragility/arc-cli/commit/caaccc52495b0ca6133efc6e96377005509c47e7))


## [0.2.3](https://github.com/bobchevalieragility/arc-cli/compare/v0.2.2...v0.2.3)

### 🐛 Bug Fixes


- Correct -day option of influx-dump - ([b186e2a](https://github.com/bobchevalieragility/arc-cli/commit/b186e2a2fb1ce9c0146d2e141ec7a9be2cbfee60))


## [0.2.2](https://github.com/bobchevalieragility/arc-cli/compare/v0.2.1...v0.2.2)

### 📚 Documentation


- Update README - ([df59fd0](https://github.com/bobchevalieragility/arc-cli/commit/df59fd0be7d7d4f0084b150227dc67d74efb7d46))


## [0.2.1](https://github.com/bobchevalieragility/arc-cli/compare/v0.2.0...v0.2.1)

### ⛰️ Features


- Add influx-dump task - ([0adbc05](https://github.com/bobchevalieragility/arc-cli/commit/0adbc05c835288897f8ed619d48d43644c39c8a1))


## [0.2.0](https://github.com/bobchevalieragility/arc-cli/compare/v0.1.4...v0.2.0)

### 🐛 Bug Fixes


- Fixed Influx URLs for prod and stage - ([94c15b0](https://github.com/bobchevalieragility/arc-cli/commit/94c15b0cd3d5dcc9881df00d5d25affcb3863132))

### 🚜 Refactor


- Add Goal constructors - ([1806cac](https://github.com/bobchevalieragility/arc-cli/commit/1806cac5c30f71b5f01e28103c01e7730f4aabfd))
- Add internal GoalParams - ([457b62f](https://github.com/bobchevalieragility/arc-cli/commit/457b62f7fbdcfa51f138fb0fa16786cc5bb8a545))
- Rename TaskType=>GoalType, and Args=>CliArgs - ([20018f1](https://github.com/bobchevalieragility/arc-cli/commit/20018f12c632c5760e671fb4a6885bcc1814946c))
- Split out args, goals, and state - ([cc43649](https://github.com/bobchevalieragility/arc-cli/commit/cc43649aea32049847ce047c29123c4c2d30cd32))

### ⚙️ Miscellaneous Tasks


- Add additional actuator services - ([3a47fed](https://github.com/bobchevalieragility/arc-cli/commit/3a47fed7fb8bbbd132b97e3a3e116241be6d6168))


## [0.1.4](https://github.com/bobchevalieragility/arc-cli/compare/v0.1.3...v0.1.4)

### ⛰️ Features


- Gracefully handle Esc key - ([aa60321](https://github.com/bobchevalieragility/arc-cli/commit/aa603216224d544c8233c4ee6531c9af27664f98))

### 🚜 Refactor


- Cleanup workflow script - ([dd47597](https://github.com/bobchevalieragility/arc-cli/commit/dd47597f738bb565870baba3d8b7ae4162f28e13))


## [0.1.3](https://github.com/bobchevalieragility/arc-cli/compare/v0.1.2...v0.1.3)

### ⛰️ Features


- Add AWS SSO task - ([e43e154](https://github.com/bobchevalieragility/arc-cli/commit/e43e1541d79eadc96264fd53e8ad185792888839))


## [0.1.2](https://github.com/bobchevalieragility/arc-cli/compare/v0.1.1...v0.1.2)

### ⛰️ Features


- Add tab completions - ([052dcf7](https://github.com/bobchevalieragility/arc-cli/commit/052dcf79f42364ce5100f098d323793c454571de))


## [0.1.1](https://github.com/bobchevalieragility/arc-cli/compare/v0.1.0...v0.1.1)

### ⛰️ Features


- Simplify AwsProfileInfo and KubeContextInfo structs - ([14127f3](https://github.com/bobchevalieragility/arc-cli/commit/14127f392ecc04c8982a6858b692bad1b7ad2018))
- Add proper error handling - ([350fb94](https://github.com/bobchevalieragility/arc-cli/commit/350fb94dda866c40f492585eed7516d2f392edc7))
- Force help messages to stderr - ([4383418](https://github.com/bobchevalieragility/arc-cli/commit/43834187f40d525a84325541a82282e7e885343c))
- Add release-plz workflow - ([7b104ce](https://github.com/bobchevalieragility/arc-cli/commit/7b104cee8f970cc084076130b71f5a21301e73e1))

### 🐛 Bug Fixes


- Allow override of service name - ([8fe66fa](https://github.com/bobchevalieragility/arc-cli/commit/8fe66faba765c33b7535dc262a1d627cb6727839))
- Retain quotes around RDS passwords - ([9ce3746](https://github.com/bobchevalieragility/arc-cli/commit/9ce3746fec24874dd1214b902b37eae7014ef0d8))
- Modify cross-compilation to use cross tool - ([01c5802](https://github.com/bobchevalieragility/arc-cli/commit/01c5802539872ff802a5b8458be00018b4430842))
- Use git tags for version comparison - ([022b840](https://github.com/bobchevalieragility/arc-cli/commit/022b8402aec761997f970f1d9a9d2b3b4eaa5711))
- Remove comment - ([f1889d8](https://github.com/bobchevalieragility/arc-cli/commit/f1889d85992634e37c1834f5e263ec7a3bd9de9f))

### 🚜 Refactor


- Create error constructors - ([7f29037](https://github.com/bobchevalieragility/arc-cli/commit/7f290372a6199c9113f248ad7707476d6e6090db))

### 📚 Documentation


- Add overview to readme - ([5d34d7c](https://github.com/bobchevalieragility/arc-cli/commit/5d34d7c1840ca462090d63d36705065ec5e5193d))
- Include development sectin in readme - ([76a79c8](https://github.com/bobchevalieragility/arc-cli/commit/76a79c8eab957d4b5875aa08ac2a7a94876ee006))

### ⚙️ Miscellaneous Tasks


- Fix wrapper shell artifact path - ([33708fd](https://github.com/bobchevalieragility/arc-cli/commit/33708fd92d8c18add2be4a7b4499dcaf30a5b065))
- Use jq to extract tag_name - ([5de147d](https://github.com/bobchevalieragility/arc-cli/commit/5de147da48ecfb773df217c79375d83286cb2bc0))
- Temporarily disable release-plz - ([e369dde](https://github.com/bobchevalieragility/arc-cli/commit/e369dde6b441c93f1f8a62f52ee86da72d81c179))
- Manually bump version - ([a0b34e4](https://github.com/bobchevalieragility/arc-cli/commit/a0b34e47f9cc6372689c119f8beefa06b9688d50))
- Update release-plz changlog format - ([c448075](https://github.com/bobchevalieragility/arc-cli/commit/c4480750d34252a5f712d77036ee32bfb57f1d9a))
- Fix release-plz.toml - ([681c870](https://github.com/bobchevalieragility/arc-cli/commit/681c870720aa7b84c55d9471e80215ea16c56725))

