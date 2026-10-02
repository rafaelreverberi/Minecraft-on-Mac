# License decision

New launcher code is GPL-3.0-only to remain compatible with the integrated Xodus GPLv3 library. LICENSE contains GPLv3. Vendored Xodus retains its original license. Existing xgameruntime and XCurl source snapshots retain MIT notices in docs/licenses; inclusion does not transfer ownership of those works. Public redistribution needs a complete dependency/source-offer inventory, including Rust/MinGW components and generated ABI probes.

No Microsoft-proprietary game files or DLLs are packaged. The app may copy user-owned local runtime DLLs into temporary self-test folders, then delete those temporary copies. It may make a local game snapshot for the owner's use. Microsoft and Minecraft names are identifiers, not a claim of affiliation or trademark rights. CrossOver is an independently installed commercial dependency.
