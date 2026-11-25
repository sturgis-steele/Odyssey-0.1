# Rust dependency inventory

Cargo.lock resolves 152 external crate versions. The table includes development, build and target-specific packages. Windows and ARM64 columns include normal and build-time dependencies. Other entries belong to tests or additional target resolutions.

The license column reproduces the packaged manifest expression. A slash is an older dual-license spelling. The file count covers packaged top-level license, copyright and notice files and license directories. Review embedded source and data notices before shipping. See [distribution considerations](distribution-considerations.md).

Regenerate with `python scripts/audit-rust-licenses.py` after `cargo fetch --locked`. Add `--notices PATH` to collect upstream text for manual review. The inventory and output include build-only dependencies as a conservative superset.

| Crate and version | Relationship | Windows | ARM64 | Declared license | Notice files |
| --- | --- | --- | --- | --- | ---: |
| [atomic-waker 1.1.2](https://crates.io/crates/atomic-waker/1.1.2) | transitive | yes | yes | Apache-2.0 OR MIT | 3 |
| [autocfg 1.5.1](https://crates.io/crates/autocfg/1.5.1) | transitive | yes | yes | Apache-2.0 OR MIT | 2 |
| [axum 0.8.9](https://crates.io/crates/axum/0.8.9) | direct | yes | yes | MIT | 1 |
| [axum-core 0.5.6](https://crates.io/crates/axum-core/0.5.6) | transitive | yes | yes | MIT | 1 |
| [base64 0.23.1](https://crates.io/crates/base64/0.23.1) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [bitflags 2.13.2](https://crates.io/crates/bitflags/2.13.2) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [bumpalo 3.20.4](https://crates.io/crates/bumpalo/3.20.4) | transitive | – | – | MIT OR Apache-2.0 | 2 |
| [bytes 1.12.2](https://crates.io/crates/bytes/1.12.2) | transitive | yes | yes | MIT | 1 |
| [cc 1.11.3](https://crates.io/crates/cc/1.11.3) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [cfg-if 1.0.5](https://crates.io/crates/cfg-if/1.0.5) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [displaydoc 0.2.7](https://crates.io/crates/displaydoc/0.2.7) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [dotenvy 0.15.7](https://crates.io/crates/dotenvy/0.15.7) | direct | yes | yes | MIT | 1 |
| [equivalent 1.0.3](https://crates.io/crates/equivalent/1.0.3) | transitive | yes | yes | Apache-2.0 OR MIT | 2 |
| [errno 0.3.15](https://crates.io/crates/errno/0.3.15) | transitive | – | yes | MIT OR Apache-2.0 | 2 |
| [fallible-iterator 0.3.0](https://crates.io/crates/fallible-iterator/0.3.0) | transitive | yes | yes | MIT/Apache-2.0 | 2 |
| [fallible-streaming-iterator 0.1.9](https://crates.io/crates/fallible-streaming-iterator/0.1.9) | transitive | yes | yes | MIT/Apache-2.0 | 2 |
| [fastrand 2.5.0](https://crates.io/crates/fastrand/2.5.0) | transitive | – | – | Apache-2.0 OR MIT | 2 |
| [find-msvc-tools 0.1.15](https://crates.io/crates/find-msvc-tools/0.1.15) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [foldhash 0.2.0](https://crates.io/crates/foldhash/0.2.0) | transitive | yes | yes | Zlib | 1 |
| [form_urlencoded 1.2.3](https://crates.io/crates/form_urlencoded/1.2.3) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [futures-channel 0.3.34](https://crates.io/crates/futures-channel/0.3.34) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [futures-core 0.3.34](https://crates.io/crates/futures-core/0.3.34) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [futures-io 0.3.34](https://crates.io/crates/futures-io/0.3.34) | transitive | – | – | MIT OR Apache-2.0 | 2 |
| [futures-macro 0.3.34](https://crates.io/crates/futures-macro/0.3.34) | transitive | – | – | MIT OR Apache-2.0 | 2 |
| [futures-sink 0.3.34](https://crates.io/crates/futures-sink/0.3.34) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [futures-task 0.3.34](https://crates.io/crates/futures-task/0.3.34) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [futures-util 0.3.34](https://crates.io/crates/futures-util/0.3.34) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [getrandom 0.4.3](https://crates.io/crates/getrandom/0.4.3) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [hashbrown 0.17.2](https://crates.io/crates/hashbrown/0.17.2) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [hashlink 0.12.2](https://crates.io/crates/hashlink/0.12.2) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [http 1.5.0](https://crates.io/crates/http/1.5.0) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [http-body 1.1.0](https://crates.io/crates/http-body/1.1.0) | transitive | yes | yes | MIT | 1 |
| [http-body-util 0.1.5](https://crates.io/crates/http-body-util/0.1.5) | transitive | yes | yes | MIT | 1 |
| [http-range-header 0.4.2](https://crates.io/crates/http-range-header/0.4.2) | transitive | yes | yes | MIT | 1 |
| [httparse 1.10.1](https://crates.io/crates/httparse/1.10.1) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [httpdate 1.0.3](https://crates.io/crates/httpdate/1.0.3) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [hyper 1.13.0](https://crates.io/crates/hyper/1.13.0) | transitive | yes | yes | MIT | 1 |
| [hyper-util 0.1.21](https://crates.io/crates/hyper-util/0.1.21) | transitive | yes | yes | MIT | 1 |
| [icu_collections 2.4.0](https://crates.io/crates/icu_collections/2.4.0) | transitive | yes | yes | Unicode-3.0 | 1 |
| [icu_locale_core 2.4.0](https://crates.io/crates/icu_locale_core/2.4.0) | transitive | yes | yes | Unicode-3.0 | 1 |
| [icu_normalizer 2.4.0](https://crates.io/crates/icu_normalizer/2.4.0) | transitive | yes | yes | Unicode-3.0 | 1 |
| [icu_normalizer_data 2.4.0](https://crates.io/crates/icu_normalizer_data/2.4.0) | transitive | yes | yes | Unicode-3.0 | 1 |
| [icu_properties 2.4.0](https://crates.io/crates/icu_properties/2.4.0) | transitive | yes | yes | Unicode-3.0 | 1 |
| [icu_properties_data 2.4.0](https://crates.io/crates/icu_properties_data/2.4.0) | transitive | yes | yes | Unicode-3.0 | 1 |
| [icu_provider 2.4.0](https://crates.io/crates/icu_provider/2.4.0) | transitive | yes | yes | Unicode-3.0 | 1 |
| [idna 1.1.0](https://crates.io/crates/idna/1.1.0) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [idna_adapter 1.2.2](https://crates.io/crates/idna_adapter/1.2.2) | transitive | yes | yes | Apache-2.0 OR MIT | 2 |
| [indexmap 2.14.2](https://crates.io/crates/indexmap/2.14.2) | transitive | yes | yes | Apache-2.0 OR MIT | 2 |
| [ipnet 2.12.2](https://crates.io/crates/ipnet/2.12.2) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [itoa 1.0.19](https://crates.io/crates/itoa/1.0.19) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [jiff 1.0.3](https://crates.io/crates/jiff/1.0.3) | direct | yes | yes | Unlicense OR MIT | 3 |
| [jiff-core 1.0.2](https://crates.io/crates/jiff-core/1.0.2) | transitive | yes | yes | Unlicense OR MIT | 3 |
| [jiff-static 1.0.3](https://crates.io/crates/jiff-static/1.0.3) | transitive | – | – | Unlicense OR MIT | 3 |
| [jiff-tzdb 1.0.0](https://crates.io/crates/jiff-tzdb/1.0.0) | transitive | yes | – | Unlicense OR MIT | 3 |
| [jiff-tzdb-platform 1.0.0](https://crates.io/crates/jiff-tzdb-platform/1.0.0) | transitive | yes | – | Unlicense OR MIT | 3 |
| [js-sys 0.3.107](https://crates.io/crates/js-sys/0.3.107) | transitive | – | – | MIT OR Apache-2.0 | 2 |
| [lazy_static 1.5.1](https://crates.io/crates/lazy_static/1.5.1) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [libc 0.2.191](https://crates.io/crates/libc/0.2.191) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [libsqlite3-sys 0.40.0](https://crates.io/crates/libsqlite3-sys/0.40.0) | transitive | yes | yes | MIT | 1 |
| [linux-raw-sys 0.12.1](https://crates.io/crates/linux-raw-sys/0.12.1) | transitive | – | – | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 4 |
| [litemap 0.8.3](https://crates.io/crates/litemap/0.8.3) | transitive | yes | yes | Unicode-3.0 | 1 |
| [log 0.4.35](https://crates.io/crates/log/0.4.35) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [matchit 0.8.4](https://crates.io/crates/matchit/0.8.4) | transitive | yes | yes | MIT AND BSD-3-Clause | 2 |
| [memchr 2.8.3](https://crates.io/crates/memchr/2.8.3) | transitive | yes | yes | Unlicense OR MIT | 3 |
| [mime 0.3.18](https://crates.io/crates/mime/0.3.18) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [mime_guess 2.0.5](https://crates.io/crates/mime_guess/2.0.5) | transitive | yes | yes | MIT | 1 |
| [mio 1.2.5](https://crates.io/crates/mio/1.2.5) | transitive | yes | yes | MIT | 1 |
| [num-bigint 0.5.1](https://crates.io/crates/num-bigint/0.5.1) | direct | yes | yes | MIT OR Apache-2.0 | 2 |
| [num-integer 0.1.47](https://crates.io/crates/num-integer/0.1.47) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [num-traits 0.2.19](https://crates.io/crates/num-traits/0.2.19) | direct | yes | yes | MIT OR Apache-2.0 | 2 |
| [once_cell 1.21.4](https://crates.io/crates/once_cell/1.21.4) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [percent-encoding 2.3.2](https://crates.io/crates/percent-encoding/2.3.2) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [pin-project-lite 0.2.18](https://crates.io/crates/pin-project-lite/0.2.18) | transitive | yes | yes | Apache-2.0 OR MIT | 2 |
| [pkg-config 0.3.34](https://crates.io/crates/pkg-config/0.3.34) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [portable-atomic 1.15.0](https://crates.io/crates/portable-atomic/1.15.0) | transitive | – | – | Apache-2.0 OR MIT | 2 |
| [portable-atomic-util 0.2.8](https://crates.io/crates/portable-atomic-util/0.2.8) | transitive | – | – | Apache-2.0 OR MIT | 2 |
| [potential_utf 0.1.6](https://crates.io/crates/potential_utf/0.1.6) | transitive | yes | yes | Unicode-3.0 | 1 |
| [proc-macro2 1.0.107](https://crates.io/crates/proc-macro2/1.0.107) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [quote 1.0.47](https://crates.io/crates/quote/1.0.47) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [r-efi 6.0.0](https://crates.io/crates/r-efi/6.0.0) | transitive | – | – | MIT OR Apache-2.0 OR LGPL-2.1-or-later | 0 |
| [reqwest 0.13.5](https://crates.io/crates/reqwest/0.13.5) | direct | yes | yes | MIT OR Apache-2.0 | 2 |
| [rsqlite-vfs 0.2.0](https://crates.io/crates/rsqlite-vfs/0.2.0) | transitive | – | – | MIT | 0 |
| [rusqlite 0.43.0](https://crates.io/crates/rusqlite/0.43.0) | direct | yes | yes | MIT | 1 |
| [rustix 1.1.6](https://crates.io/crates/rustix/1.1.6) | transitive | – | – | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 4 |
| [rustversion 1.0.23](https://crates.io/crates/rustversion/1.0.23) | transitive | – | – | MIT OR Apache-2.0 | 2 |
| [ryu 1.0.23](https://crates.io/crates/ryu/1.0.23) | transitive | yes | yes | Apache-2.0 OR BSL-1.0 | 2 |
| [serde 1.0.229](https://crates.io/crates/serde/1.0.229) | direct | yes | yes | MIT OR Apache-2.0 | 2 |
| [serde_core 1.0.229](https://crates.io/crates/serde_core/1.0.229) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [serde_derive 1.0.229](https://crates.io/crates/serde_derive/1.0.229) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [serde_json 1.0.154](https://crates.io/crates/serde_json/1.0.154) | direct | yes | yes | MIT OR Apache-2.0 | 2 |
| [serde_path_to_error 0.1.20](https://crates.io/crates/serde_path_to_error/0.1.20) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [serde_urlencoded 0.7.1](https://crates.io/crates/serde_urlencoded/0.7.1) | transitive | yes | yes | MIT/Apache-2.0 | 2 |
| [sharded-slab 0.1.7](https://crates.io/crates/sharded-slab/0.1.7) | transitive | yes | yes | MIT | 1 |
| [shlex 2.0.1](https://crates.io/crates/shlex/2.0.1) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [signal-hook-registry 1.4.8](https://crates.io/crates/signal-hook-registry/1.4.8) | transitive | – | yes | MIT OR Apache-2.0 | 2 |
| [slab 0.4.12](https://crates.io/crates/slab/0.4.12) | transitive | yes | yes | MIT | 1 |
| [smallvec 1.16.6](https://crates.io/crates/smallvec/1.16.6) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [socket2 0.6.5](https://crates.io/crates/socket2/0.6.5) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [sqlite-wasm-rs 0.6.3](https://crates.io/crates/sqlite-wasm-rs/0.6.3) | transitive | – | – | MIT | 1 |
| [stable_deref_trait 1.2.1](https://crates.io/crates/stable_deref_trait/1.2.1) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [syn 2.0.119](https://crates.io/crates/syn/2.0.119) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [syn 3.0.10](https://crates.io/crates/syn/3.0.10) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [sync_wrapper 1.0.2](https://crates.io/crates/sync_wrapper/1.0.2) | transitive | yes | yes | Apache-2.0 | 1 |
| [synstructure 0.14.0](https://crates.io/crates/synstructure/0.14.0) | transitive | yes | yes | MIT | 1 |
| [tempfile 3.27.0](https://crates.io/crates/tempfile/3.27.0) | dev | – | – | MIT OR Apache-2.0 | 2 |
| [thiserror 2.0.21](https://crates.io/crates/thiserror/2.0.21) | direct | yes | yes | MIT OR Apache-2.0 | 2 |
| [thiserror-impl 2.0.21](https://crates.io/crates/thiserror-impl/2.0.21) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [thread_local 1.1.10](https://crates.io/crates/thread_local/1.1.10) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [tinystr 0.8.5](https://crates.io/crates/tinystr/0.8.5) | transitive | yes | yes | Unicode-3.0 | 1 |
| [tokio 1.54.1](https://crates.io/crates/tokio/1.54.1) | direct | yes | yes | MIT | 1 |
| [tokio-macros 2.7.2](https://crates.io/crates/tokio-macros/2.7.2) | transitive | yes | yes | MIT | 1 |
| [tokio-stream 0.1.19](https://crates.io/crates/tokio-stream/0.1.19) | direct | yes | yes | MIT | 1 |
| [tokio-util 0.7.20](https://crates.io/crates/tokio-util/0.7.20) | transitive | yes | yes | MIT | 1 |
| [tower 0.5.3](https://crates.io/crates/tower/0.5.3) | transitive | yes | yes | MIT | 1 |
| [tower-http 0.6.11](https://crates.io/crates/tower-http/0.6.11) | direct | yes | yes | MIT | 1 |
| [tower-layer 0.3.3](https://crates.io/crates/tower-layer/0.3.3) | transitive | yes | yes | MIT | 1 |
| [tower-service 0.3.3](https://crates.io/crates/tower-service/0.3.3) | transitive | yes | yes | MIT | 1 |
| [tracing 0.1.44](https://crates.io/crates/tracing/0.1.44) | direct | yes | yes | MIT | 1 |
| [tracing-attributes 0.1.31](https://crates.io/crates/tracing-attributes/0.1.31) | transitive | yes | yes | MIT | 1 |
| [tracing-core 0.1.36](https://crates.io/crates/tracing-core/0.1.36) | transitive | yes | yes | MIT | 1 |
| [tracing-serde 0.2.0](https://crates.io/crates/tracing-serde/0.2.0) | transitive | yes | yes | MIT | 1 |
| [tracing-subscriber 0.3.23](https://crates.io/crates/tracing-subscriber/0.3.23) | direct | yes | yes | MIT | 1 |
| [try-lock 0.2.5](https://crates.io/crates/try-lock/0.2.5) | transitive | yes | yes | MIT | 1 |
| [unicase 2.10.0](https://crates.io/crates/unicase/2.10.0) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [unicode-ident 1.0.27](https://crates.io/crates/unicode-ident/1.0.27) | transitive | yes | yes | (MIT OR Apache-2.0) AND Unicode-3.0 | 3 |
| [url 2.5.9](https://crates.io/crates/url/2.5.9) | transitive | yes | yes | MIT OR Apache-2.0 | 2 |
| [utf8_iter 1.1.0](https://crates.io/crates/utf8_iter/1.1.0) | transitive | yes | yes | Apache-2.0 OR MIT | 3 |
| [utf_types 1.0.0](https://crates.io/crates/utf_types/1.0.0) | transitive | yes | yes | Apache-2.0 OR MIT | 3 |
| [uuid 1.29.3](https://crates.io/crates/uuid/1.29.3) | direct | yes | yes | Apache-2.0 OR MIT | 2 |
| [valuable 0.1.1](https://crates.io/crates/valuable/0.1.1) | transitive | – | – | MIT | 0 |
| [vcpkg 0.2.15](https://crates.io/crates/vcpkg/0.2.15) | transitive | yes | yes | MIT/Apache-2.0 | 2 |
| [want 0.3.2](https://crates.io/crates/want/0.3.2) | transitive | yes | yes | MIT | 1 |
| [wasi 0.11.1+wasi-snapshot-preview1](https://crates.io/crates/wasi/0.11.1+wasi-snapshot-preview1) | transitive | – | – | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 3 |
| [wasm-bindgen 0.2.130](https://crates.io/crates/wasm-bindgen/0.2.130) | transitive | – | – | MIT OR Apache-2.0 | 2 |
| [wasm-bindgen-futures 0.4.80](https://crates.io/crates/wasm-bindgen-futures/0.4.80) | transitive | – | – | MIT OR Apache-2.0 | 2 |
| [wasm-bindgen-macro 0.2.130](https://crates.io/crates/wasm-bindgen-macro/0.2.130) | transitive | – | – | MIT OR Apache-2.0 | 2 |
| [wasm-bindgen-macro-support 0.2.130](https://crates.io/crates/wasm-bindgen-macro-support/0.2.130) | transitive | – | – | MIT OR Apache-2.0 | 2 |
| [wasm-bindgen-shared 0.2.130](https://crates.io/crates/wasm-bindgen-shared/0.2.130) | transitive | – | – | MIT OR Apache-2.0 | 2 |
| [wasm-streams 0.5.0](https://crates.io/crates/wasm-streams/0.5.0) | transitive | – | – | MIT OR Apache-2.0 | 2 |
| [web-sys 0.3.107](https://crates.io/crates/web-sys/0.3.107) | transitive | – | – | MIT OR Apache-2.0 | 2 |
| [windows-link 0.2.1](https://crates.io/crates/windows-link/0.2.1) | transitive | yes | – | MIT OR Apache-2.0 | 2 |
| [windows-sys 0.61.2](https://crates.io/crates/windows-sys/0.61.2) | transitive | yes | – | MIT OR Apache-2.0 | 2 |
| [winreg 0.56.0](https://crates.io/crates/winreg/0.56.0) | direct | yes | – | MIT | 1 |
| [writeable 0.6.4](https://crates.io/crates/writeable/0.6.4) | transitive | yes | yes | Unicode-3.0 | 1 |
| [yoke 0.8.4](https://crates.io/crates/yoke/0.8.4) | transitive | yes | yes | Unicode-3.0 | 1 |
| [yoke-derive 0.8.4](https://crates.io/crates/yoke-derive/0.8.4) | transitive | yes | yes | Unicode-3.0 | 1 |
| [zerofrom 0.1.8](https://crates.io/crates/zerofrom/0.1.8) | transitive | yes | yes | Unicode-3.0 | 1 |
| [zerofrom-derive 0.1.8](https://crates.io/crates/zerofrom-derive/0.1.8) | transitive | yes | yes | Unicode-3.0 | 1 |
| [zerotrie 0.2.5](https://crates.io/crates/zerotrie/0.2.5) | transitive | yes | yes | Unicode-3.0 | 1 |
| [zerovec 0.11.9](https://crates.io/crates/zerovec/0.11.9) | transitive | yes | yes | Unicode-3.0 | 1 |
| [zerovec-derive 0.11.7](https://crates.io/crates/zerovec-derive/0.11.7) | transitive | yes | yes | Unicode-3.0 | 1 |
| [zmij 1.0.23](https://crates.io/crates/zmij/1.0.23) | transitive | yes | yes | MIT | 1 |
