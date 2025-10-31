To generate the log, run `git log --pretty='* %h - %s (%an, %ad)' TAGNAME..HEAD .` replacing TAGNAME and HEAD as appropriate.

# 0.7.0 - [Feature Release]

* 1c42707 - chore: rename bin.rs to bin/pact-stub-server (Yousaf Nabi, Fri Oct 31 16:01:18 2025 +0000)
* ac5344e - fix: remove pathname from release artifact shasum (Yousaf Nabi, Fri Oct 24 13:56:27 2025 +0100)
* 1ddbc54 - chore(ci): fix release workflow (macos) (Yousaf Nabi, Fri Oct 24 13:53:26 2025 +0100)
* 793fa3b - chore(ci): fix release workflow (macos) (Yousaf Nabi, Fri Oct 24 13:52:17 2025 +0100)
* 45e0056 - feat: add file watch mode for hot reloading pact files/dirs (Yousaf Nabi, Thu Oct 23 23:43:13 2025 +0100)
* eebe3fe - feat: update to hyper 1.x / latest tower (Yousaf Nabi, Thu Oct 23 23:35:30 2025 +0100)
* a4f02ae - chore: rename main.rs to lib.rs to align with rust conventions (Yousaf Nabi, Wed Oct 8 22:19:35 2025 +0100)
* a36631c - ci: switch windows-2019 to windows-latest (Yousaf Nabi, Sat Sep 27 11:50:22 2025 +0100)
* 56ccc6a - feat: cli as lib (Yousaf Nabi, Sat Sep 27 11:47:46 2025 +0100)
* 7548780 - chore: Correct the release script (Ronald Holshausen, Wed May 21 16:07:31 2025 +1000)
* 92ce531 - bump version to 0.6.3 (Ronald Holshausen, Wed May 21 15:27:53 2025 +1000)

# 0.6.2 - Correct the insecure_tls flag with a Pact Broker

* 1a51ae7 - chore: Update pact_verifier to 1.3.0 (Ronald Holshausen, Wed May 21 15:12:44 2025 +1000)
* a0fd845 - fix: Honour the insecure_tls flag when fetching Pacts from a Pact Broker #75 (Ronald Holshausen, Tue May 20 16:28:25 2025 +1000)
* 1b2580f - chore: Upgrade the Pact crates to latest versions (Ronald Holshausen, Tue May 20 15:40:59 2025 +1000)
* 85fd8f5 - chore: Upgrade the Pact crates (Ronald Holshausen, Tue May 20 15:32:30 2025 +1000)
* 95e253f - chore: Update project to Rust 2024 edition (Ronald Holshausen, Tue May 20 10:34:35 2025 +1000)
* 9665d36 - chore: Update dependencies (Ronald Holshausen, Tue May 20 10:32:49 2025 +1000)
* 0d39f57 - chore: scheduled Ubuntu 20.04 retirement (Ronald Holshausen, Mon May 12 09:46:13 2025 +1000)
* f4e42b4 - chore: Correct the author field (Ronald Holshausen, Mon May 12 09:21:57 2025 +1000)
* 6269fa6 - bump version to 0.6.2 (Ronald Holshausen, Mon May 12 09:20:30 2025 +1000)

# 0.6.1 - Performance Improvements

* 7188299 - perf: Use MiMalloc as global allocator to improve performance (Stefan Ansing, Tue May 6 16:15:21 2025 +0200)
* ef0e8e4 - perf: Move shared data into Arc (Stefan Ansing, Tue May 6 16:12:28 2025 +0200)
* 43437ac - chore(ci): upgrade macos-12 to macos-13 (JP-Ellis, Fri Dec 6 09:33:47 2024 +1100)
* c5383f4 - chore(docs): typo in link name (Yousaf Nabi, Mon May 20 13:14:27 2024 +0100)
* bc78875 - chore(docs): update badge link to correct repo (Yousaf Nabi, Mon May 20 13:13:33 2024 +0100)
* 7772232 - docs: docker/binary compatibility notes (Yousaf Nabi, Mon May 20 13:12:37 2024 +0100)
* 7a7094b - bump version to 0.6.1 (Yousaf Nabi, Fri May 10 16:23:21 2024 +0100)

# 0.6.0 - Feature Release

* 9387eb1 - chore(ci): remove cache workspace subdir (Yousaf Nabi, Fri Apr 26 15:49:13 2024 +0100)
* 2325de3 - feat: reduce executable size (Yousaf Nabi, Thu Apr 25 19:06:53 2024 +0100)
* efa86ca - feat: linux musl static bins / windows aarch64 (Yousaf Nabi, Thu Apr 25 17:56:00 2024 +0100)

# 0.5.3 - add regex support with consumer/provider name filtering

* 2229a40 - feat: correct retrieving regex values from CLI args #54 (Ronald Holshausen, Tue Apr 11 13:55:16 2023 +1000)
* cb982da - feat: add regex support with consumer/provider name filtering #54 (Ronald Holshausen, Tue Apr 11 13:43:27 2023 +1000)
* 4cea830 - chore: fix deprecation warnings (Ronald Holshausen, Tue Apr 11 12:07:23 2023 +1000)
* 85bc42f - chore: Upgrade clap to latest 4.2 (Ronald Holshausen, Tue Apr 11 12:03:31 2023 +1000)
* 1233cf8 - chore: add trycmd tests (Ronald Holshausen, Tue Apr 11 11:04:43 2023 +1000)
* 6dfb360 - chore: Upgrade dependencies (Ronald Holshausen, Tue Apr 11 10:42:55 2023 +1000)
* 257bd0f - chore: Update docker image to support multi-arch images (Ronald Holshausen, Tue Oct 25 12:48:12 2022 +1100)
* b5b1ee2 - Merge branch 'release/0.5.2' (Ronald Holshausen, Tue Oct 25 11:48:26 2022 +1100)
* c4fbdfd - chore: update release script (Ronald Holshausen, Mon Oct 24 17:49:38 2022 +1100)
* 858193f - bump version to 0.5.3 (Ronald Holshausen, Mon Oct 24 17:45:42 2022 +1100)

# 0.5.2 - Support linux with glibc 2.18+ and aarch64 (ARM64) binaries

* 2322656 - fix: file extension option should only be given once (Ronald Holshausen, Mon Oct 24 17:14:50 2022 +1100)
* ab2c414 - chore: make the provider and consumer name parameters singular (Ronald Holshausen, Mon Oct 24 17:12:06 2022 +1100)
* 5398be5 - chore: correct manifest homepage (Ronald Holshausen, Mon Oct 24 16:47:58 2022 +1100)
* 2e4c46e - chore: update dep crates (Ronald Holshausen, Mon Oct 24 16:47:21 2022 +1100)
* cbf1ebf - chore: add test with interactions with different query parameters (Ronald Holshausen, Mon Oct 24 16:21:07 2022 +1100)
* c2be7b1 - chore: Add aarch64 (ARM64) binaries to the release build #50 (Ronald Holshausen, Mon Oct 24 15:19:46 2022 +1100)
* 40c20ff - chore: Update main doc comment (Ronald Holshausen, Mon Oct 24 15:11:56 2022 +1100)
* fde3606 - chore: Upgrade Clap to 4.0.x (Ronald Holshausen, Mon Oct 24 15:05:05 2022 +1100)
* ca3c5d0 - fix: build linux executable with Debian Stretch (supports GLibC 2.18+) #39 (Ronald Holshausen, Mon Oct 24 11:28:37 2022 +1100)
* 5bee6ed - chore: Update dependencies (Ronald Holshausen, Mon Oct 24 11:10:46 2022 +1100)
* b9a81b2 - chore: add readme (Ronald Holshausen, Fri Jun 10 16:45:03 2022 +1000)
* c7db05a - bump version to 0.5.2 (Ronald Holshausen, Fri Jun 10 16:24:42 2022 +1000)

# 0.5.1 - Bugfix Release

* d864952 - fix: Upgrade pact_matching crate to 0.12.9 (fixes type matches with query parameters) #48 (Ronald Holshausen, Fri Jun 10 15:49:30 2022 +1000)
* 62fd1ac - chore: update docker file (Ronald Holshausen, Wed Jun 8 16:17:13 2022 +1000)
* bb60af3 - bump version to 0.5.1 (Ronald Holshausen, Wed Jun 8 15:54:02 2022 +1000)

# 0.5.0 - Fixes + upgrade to V4 Pact

* 2abfdcb - chore: update readme (Ronald Holshausen, Wed Jun 8 15:37:22 2022 +1000)
* 4004db6 - chore: fix failing test #46 (Ronald Holshausen, Wed Jun 8 15:23:13 2022 +1000)
* baa620a - feat: add filters for consumer and provider names #46 (Ronald Holshausen, Wed Jun 8 15:21:38 2022 +1000)
* 7278b8f - chore: Upgrade clap to 3.0.x (Ronald Holshausen, Wed Jun 8 13:52:10 2022 +1000)
* 5dab424 - chore: Convert logging to the tracing crate (Ronald Holshausen, Wed Jun 8 11:46:55 2022 +1000)
* e193558 - feat: Upgrade all Pact crates to support V4 and async (Ronald Holshausen, Wed Jun 8 11:28:57 2022 +1000)
* da9d7ab - chore: bump minor version (Ronald Holshausen, Tue Jun 7 13:37:54 2022 +1000)
* 7c84b7e - chore: Upgrade all dependant crates (Ronald Holshausen, Tue Jun 7 13:34:48 2022 +1000)
* 4150660 - fix: remove linked openssl from application binary #47 (Ronald Holshausen, Tue Jun 7 13:33:46 2022 +1000)
* 7333efe - chore: upgrade project to Rust 2021 (Ronald Holshausen, Wed Jan 5 16:07:13 2022 +1100)
* 69722f2 - Merge pull request #45 from counterbeing/master (Ronald Holshausen, Mon Nov 22 15:43:53 2021 +1100)
* b0789ee - builds successfully on arm based m1 mac (Cory Logan, Sat Nov 20 11:55:02 2021 -0800)
* b24ee5a - fix: docker/Dockerfile to reduce vulnerabilities (snyk-bot, Sat Sep 4 03:57:21 2021 +0000)
* f4e871f - Add rustfmt configuration (Jorge Ortiz-Fuentes, Tue Apr 13 13:07:33 2021 +0200)
* 6ab7cf4 - Revert "Revert "bump version to 0.4.5"" (Ronald Holshausen, Wed Jan 27 15:18:56 2021 +1100)
* 38e041c - chore: update readme (Ronald Holshausen, Tue Jan 26 13:06:55 2021 +1100)
* 25a2a64 - chore: add tag to docker build hook (Ronald Holshausen, Tue Jan 26 12:25:45 2021 +1100)
* 50b2b6f - chore: add docker build (Ronald Holshausen, Tue Jan 26 12:18:57 2021 +1100)

# 0.4.4 - option to fetch pacts from Pact broker

* 0d7ab70 - fix: correct cargo manefest dependency version (Ronald Holshausen, Mon Jan 25 10:24:26 2021 +1100)
* a5ec95d - feat: added option to fetch pacts from Pact broker (Ronald Holshausen, Sun Jan 24 18:22:48 2021 +1100)
* 5b080b8 - fix: correct cargo manefest (Ronald Holshausen, Sun Jan 24 14:38:54 2021 +1100)
* 457759b - chore: upgrade to Tokio 1.0 and Hyper 0.14 (Ronald Holshausen, Sun Jan 24 14:28:23 2021 +1100)
* 9717506 - bump version to 0.4.4 (Ronald Holshausen, Sun Jul 26 12:42:57 2020 +1000)

# 0.4.3 - Performance optmisation

* f9515fd - fix: tests after performance optimisation (Ronald Holshausen, Sun Jul 26 12:34:11 2020 +1000)
* 998ce51 - feat: filter interactions by method and path first (Ronald Holshausen, Sun Jul 26 12:18:50 2020 +1000)
* a48d40e - feat: re-enable HTTP keepalive and get all interactions when server starts (Ronald Holshausen, Sun Jul 26 11:50:46 2020 +1000)
* 6a2d2bf - fix: when loading files from a directory, only load json files (Ronald Holshausen, Sun Jul 26 10:24:55 2020 +1000)
* d128a65 - bump version to 0.4.3 (Ronald Holshausen, Tue Jul 14 11:10:49 2020 +1000)

# 0.4.2 - Fix concurrency issue

* 20ea51f - fix: start the hyper server on a blocking thread (Ronald Holshausen, Tue Jul 14 10:49:37 2020 +1000)
* 558baae - fix: tests after updating crates (Ronald Holshausen, Tue Jul 14 10:29:18 2020 +1000)
* 6884340 - chore: update crates (Ronald Holshausen, Tue Jul 14 10:07:02 2020 +1000)
* b8fef54 - bump version to 0.4.2 (Ronald Holshausen, Fri Jun 12 13:13:21 2020 +1000)

# 0.4.1 - Update to latest Pact matching crate

* 9fad496 - feat: update to latest pact matching crate (Ronald Holshausen, Fri Jun 12 13:03:50 2020 +1000)
* 4d7e464 - chore: make --empty-provider-state flag require --provider-state parameter (Ronald Holshausen, Fri Jun 12 11:48:17 2020 +1000)
* 729674c - feat: add option to include empty provider states when using a filter #34 (Ronald Holshausen, Fri Jun 12 11:42:41 2020 +1000)
* 17e8b3b - chore: updated dependencies to latest + code cleanup (Ronald Holshausen, Fri Jun 12 11:03:30 2020 +1000)
* 3083fa1 - chore: upgrade crates and cleanup some imports (Ronald Holshausen, Thu May 21 13:17:48 2020 +1000)
* ce7093f - bump version to 0.4.1 (Ronald Holshausen, Sun Apr 5 17:14:31 2020 +1000)

# 0.4.0 - Upgrade hyper to 0.13 and Rust async/await

* 07e2d9c - chore: Upgrade hyper to 0.13 and Rust async/await (Ronald Holshausen, Sun Apr 5 16:12:34 2020 +1000)
* 2edd291 - chore: upgrade all crates and Rust to 2018 edition (Ronald Holshausen, Mon Mar 23 16:38:05 2020 +1100)
* 772fb19 - fix: GCC is not available for i686 targets on Appveyor (Ronald Holshausen, Sat Jan 18 12:55:37 2020 +1100)
* f234e2e - bump version to 0.3.3 (Ronald Holshausen, Sat Jan 18 12:48:23 2020 +1100)

# 0.3.2 - CORS referer option

* d69aa92 - feat: update readme with new cors flag #32 (Ronald Holshausen, Sat Jan 18 12:34:28 2020 +1100)
* e1865fb - feat: add option to set cors origin to the referer header #32 (Ronald Holshausen, Sat Jan 18 12:24:04 2020 +1100)
* 92f0a32 - chore: update readme (Ronald Holshausen, Sun Aug 11 11:28:46 2019 +1000)
* fa41bd5 - bump version to 0.3.2 (Ronald Holshausen, Sun Aug 11 11:20:25 2019 +1000)

# 0.3.1 - bearer tokens and headers with multiple values

* a93f256 - feat: add support for bearer tokens (Ronald Holshausen, Sun Aug 11 11:07:19 2019 +1000)
* be2a3f4 - fix: support headers with multiple values #31 (Ronald Holshausen, Sun Aug 11 10:26:21 2019 +1000)
* 71e52b6 - Setting wildcard value * for Access-Control-Allow-Headers (Dario Banfi, Sun Jul 28 17:02:52 2019 +0200)
* 934b748 - bump version to 0.3.1 (Ronald Holshausen, Sat Jun 29 20:07:48 2019 +1000)

# 0.3.0 - Bugfix Release

* 0e73a10 - chore: upgrade crates (Ronald Holshausen, Sat Jun 29 19:51:17 2019 +1000)
* e741007 - fix: upgrade to latest pact matching library (Ronald Holshausen, Sat Jun 29 19:43:12 2019 +1000)
* 990587b - fix: panic if provider_state_header_name is not given (Ronald Holshausen, Sat Jun 29 17:44:31 2019 +1000)
* 3c0e848 - Added changes requested by @uglyog in #28. (Zakaria Boutami, Wed Jun 19 15:04:51 2019 +0200)
* 31e4b5f - update changelog for release 0.3.0 (Zakaria Boutami, Tue Jun 11 16:48:31 2019 +0200)
* 0daedac - Added support for parsing the provider state from request header using a custom name. (Zakaria Boutami, Tue Jun 11 16:00:26 2019 +0200)
* 828aaa7 - bump version to 0.2.3 (Ronald Holshausen, Sun Mar 3 16:45:05 2019 +1100)

# 0.3.0 - Provider state as request header parameter

* 0daedac - Added support for parsing the provider state from request header using a custom name. (Zakaria Boutami, Tue Jun 11 16:00:34 
2019 +0200)

# 0.2.2 - Disabling TLS cert validation and filtering by provider state

* df12a27 - feat: add a filter by provider state #19 (Ronald Holshausen, Sun Mar 3 16:21:42 2019 +1100)
* 7d0bc26 - feat: updated readme about flag to disable TLS cert validation #27 (Ronald Holshausen, Sun Mar 3 15:11:27 2019 +1100)
* 849c10a - feat: added a flag to disable TLS cert validation #27 (Ronald Holshausen, Sun Mar 3 15:04:36 2019 +1100)
* 50298e6 - fix: make warning message more explicit about what it is doing #24 (Ronald Holshausen, Sat Mar 2 12:27:18 2019 +1100)
* 4c8285f - chore: removed pmacro as rust has a standard dbg macro now (Ronald Holshausen, Sat Mar 2 12:25:15 2019 +1100)
* fbd8d5b - bump version to 0.2.2 (Ronald Holshausen, Sun Jan 6 15:25:08 2019 +1100)

# 0.2.1 - Bugfix Release

* f5870d5 - fix: upgraded pact matching to 0.5.0 and corrected logging #22 #21 #20 (Ronald Holshausen, Sun Jan 6 15:17:23 2019 +1100)
* 2333cf6 - bump version to 0.2.1 (Ronald Holshausen, Mon Nov 5 15:59:22 2018 +1100)

# 0.2.0 - Bugfix Release

* 40f83a2 - chore: bump version to next minor (Ronald Holshausen, Mon Nov 5 15:48:16 2018 +1100)
* 0699283 - fix: Use a chain of futures so reading the body does not block the event loop #18 #16 (Ronald Holshausen, Mon Nov 5 15:39:49 2018 +1100)
* 075e391 - refactor: split the server code into its own module (Ronald Holshausen, Sat Oct 20 12:21:56 2018 +1100)
* de67be8 - Respect 'auto_cors' even if there is no match (Sebastian Thiel, Fri Oct 12 08:46:26 2018 +0200)
* 616c02a - bump version to 0.1.2 (Ronald Holshausen, Sat Sep 8 15:54:23 2018 +1000)

# 0.1.1 - Bugfix Release

* 7e1e64e - fix: only add a cors origin header if there is not one #15 (Ronald Holshausen, Sat Sep 8 15:37:32 2018 +1000)
* 6f110ed - fix: add some tests around content type header #14 (Ronald Holshausen, Sat Sep 8 15:25:37 2018 +1000)
* 180f30d - fix: remove static content type header #14 (Ronald Holshausen, Sat Sep 8 14:50:26 2018 +1000)
* bbc91a9 - bump version to 0.1.1 (Ronald Holshausen, Sat Aug 25 21:45:14 2018 +1000)

# 0.1.0 - Support for loading pacts from HTTPS

* a2af4ef - doc: update readme (Ronald Holshausen, Sat Aug 25 21:11:15 2018 +1000)
* 8da3496 - feat: bump minor version (Ronald Holshausen, Sat Aug 25 21:02:26 2018 +1000)
* 1f2855e - feat: implemented support for fetching pacts using HTTPS #13 (Ronald Holshausen, Sat Aug 25 20:58:32 2018 +1000)
* 5bb6dbd - refactor: Upgrade hyper crate to 0.12 #13 (Ronald Holshausen, Sat Aug 25 17:32:38 2018 +1000)
* b5cdeb2 - bump version to 0.0.11 (Ronald Holshausen, Sat Aug 11 15:33:18 2018 +1000)

# 0.0.10 - Bugfix Release

* 7f17c68 - fix: update to pact-matching 0.4.4 (Ronald Holshausen, Sat Aug 11 15:25:07 2018 +1000)
* e369261 - bump version to 0.0.10 (Ronald Holshausen, Sat Jun 30 17:27:45 2018 +1000)

# 0.0.9 - Bugfix Release

* 4b66646 - fix: upgrade the pact matching to support query parameters with path expressions #11 (Ronald Holshausen, Sat Jun 30 17:19:39 2018 +1000)
* 65c32e2 - doc: updated the readme (Ronald Holshausen, Sun May 13 15:45:50 2018 +1000)
* a6137e8 - chore: update appveyor build to use rustup (Ronald Holshausen, Sun May 13 15:38:56 2018 +1000)
* 5424e27 - bump version to 0.0.9 (Ronald Holshausen, Sun May 13 15:37:56 2018 +1000)

# 0.0.8 - Upgrade to V3 spec + bugfixes

* 4f1a3fe - fix: for PUT, POST and PATCH requests, return the first response if there is no body #10 (Ronald Holshausen, Sun May 13 15:02:45 2018 +1000)
* f3ffc83 - fix: for PUT, POST and PATCH requests, also check the body of the request #10 (Ronald Holshausen, Sun May 13 14:51:29 2018 +1000)
* 1780c3d - Moved the tests to a seperate file (Ronald Holshausen, Fri May 11 08:25:07 2018 +1000)
* ef8ec2c - Merge pull request #7 from stones/fix/cors-content-type-headers (Ronald Holshausen, Mon Nov 13 15:15:25 2017 +1100)
* 406bdd0 - Merge pull request #6 from stepan-leibo/patch-1 (Ronald Holshausen, Mon Nov 13 15:14:20 2017 +1100)
* a726e20 - bump version to 0.0.8 (Ronald Holshausen, Mon Nov 13 12:35:03 2017 +1100)
* 1d5c076 - Added 'Content-Type' to allowed headers to allow POST requests to have json bodies (Tom Stones, Fri Nov 3 15:01:41 2017 +1100)
* 627093a - Split release-osx into separate osx/ios shell scripts (Tom Stones, Fri Nov 3 14:52:55 2017 +1100)
* dbb239d - Fix online rust docs (Stepan Leibo, Thu Oct 26 14:50:44 2017 +0100)

# 0.0.7 - Bugfix Release

* 864fd12 - Update to support the changes in pact_matching 0.3.1 (Ronald Holshausen, Mon Nov 13 12:24:14 2017 +1100)
* b309cfa - bump version to 0.0.7 (Ronald Holshausen, Mon Oct 23 09:39:44 2017 +1100)

# 0.0.6 - Updated pact_matching to latest version

* 04d5416 - Updated release script (Ronald Holshausen, Mon Oct 23 09:31:04 2017 +1100)
* 35f0cea - Updated pact_matching to latest version (Ronald Holshausen, Mon Oct 23 09:29:43 2017 +1100)
* d358195 - bump version to 0.0.6 (Ronald Holshausen, Sun Sep 24 11:16:16 2017 +1000)

# 0.0.5 - Additional CORS headers

* 6c910d9 - Added more cors headers to the options request (Ronald Holshausen, Sun Sep 24 10:02:57 2017 +1000)
* 89c0658 - bump version to 0.0.5 (Ronald Holshausen, Thu Sep 21 10:05:32 2017 +1000)

# 0.0.4 - Corrected CORS request method

* ff47315 - Changed 'OPTION' request method to 'OPTIONS' ... and updated tests (Tom Stones, Thu Sep 21 07:52:39 2017 +1000)
* e0259aa - Update readme (Ronald Holshausen, Wed Sep 20 10:14:44 2017 +1000)
* 4e563ca - small code cleanup (Ronald Holshausen, Wed Sep 20 10:11:37 2017 +1000)
* 460c8b6 - bump version to 0.0.4 (Ronald Holshausen, Wed Sep 20 10:08:09 2017 +1000)

# 0.0.3 - Add option to auto-respond to CORS pre-flight requests

* a162160 - Update appveyor build to use latest rust (Ronald Holshausen, Wed Sep 20 09:50:23 2017 +1000)
* bb7e4c0 - Add auto handling of CORS pre-flight requests (Ronald Holshausen, Wed Sep 20 09:27:36 2017 +1000)
* 331a590 - Update crates to later versions (Ronald Holshausen, Wed Sep 20 08:53:41 2017 +1000)
* a106a3a - bump version to 0.0.3 (Ronald Holshausen, Thu May 4 11:44:09 2017 +1000)

# 0.0.2 - Bugfix Release

* 280de38 - Upgraded simple_log crate to 0.4.2 and switch to a simple logger if the term logger fails dueto there not being a terminal #2 (Ronald Holshausen, Thu May 4 11:36:30 2017 +1000)
* b7819ac - bump version to 0.0.2 (Ronald Holshausen, Wed Oct 26 15:01:50 2016 +1100)

# 0.0.1 - return the closest matching interaction, based on the body and headers

* fff769d - return the closest matching interaction, based on the body and headers (Ronald Holshausen, Wed Oct 26 14:55:02 2016 +1100)
* c439766 - Fix build to work with pact_matching v0.2.1 (Ronald Holshausen, Wed Oct 12 17:14:50 2016 +1100)
* a3b970e - added correct URL for appveyor badge (Ronald Holshausen, Wed Oct 5 20:18:50 2016 +1100)
* 56148eb - add appveyor build (Ronald Holshausen, Wed Oct 5 20:14:57 2016 +1100)
* 145126e - add travis badge to readme (Ronald Holshausen, Wed Oct 5 20:10:55 2016 +1100)
* c8c833c - added travis build (Ronald Holshausen, Wed Oct 5 20:08:05 2016 +1100)
* dc46c62 - correct the doco wrt logging defaulting to info (Ronald Holshausen, Wed Oct 5 20:06:28 2016 +1100)
* b3213fa - correct the release osx script (Ronald Holshausen, Wed Oct 5 20:03:35 2016 +1100)
* 7610470 - bump version to 0.0.1 (Ronald Holshausen, Wed Oct 5 16:17:46 2016 +1100)
* 5da0a49 - correct repo url and release script artifacts (Ronald Holshausen, Wed Oct 5 16:16:43 2016 +1100)
* 7c01a45 - updated tags in release script (Ronald Holshausen, Wed Oct 5 16:11:05 2016 +1100)

# 0.0.0 - First Release


##
