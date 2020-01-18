To generate the log, run `git log --pretty='* %h - %s (%an, %ad)' TAGNAME..HEAD .` replacing TAGNAME and HEAD as appropriate.

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
