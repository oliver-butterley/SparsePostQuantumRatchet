# Aeneas extraction status

## Modules that extract cleanly (no changes needed)

- `spqr::util`
- `spqr::serialize`

## Modules that extract cleanly (with modifications)

### `spqr::encoding`

Excluded (platform-specific, not needed for verification):
- `encoding::gf::check_accelerated` -- excluded
- `encoding::gf::accelerated` -- excluded

Marked as opaque:
- `encoding::gf::mul2_u16` -- opaque
- `encoding::gf::MulAssign<&GF16> for GF16` -- opaque
- `encoding::polynomial::PolyDecoder::decoded_message` -- opaque

Modified source (replaced iterator combinators / early returns with index loops):
- `encoding::gf::GF16::div_impl` -- avoid opaque tuple return
- `encoding::polynomial::Poly::compute_at` -- zip to index loop
- `encoding::polynomial::Poly::lagrange_sum` -- zip to index loop
- `encoding::polynomial::Poly::from_complete_points` -- map/collect to push loop
- `encoding::polynomial::Poly::deserialize` -- chunks_exact to while loop
- `encoding::polynomial::PolyEncoder::from_pb` -- flatten if/else, remove closure
- `encoding::polynomial::PolyDecoder::from_pb` -- hoist validation before loop

### `spqr::incremental_mlkem768`

Excluded:
- `log` -- log crate internals cause Aeneas internal error (arrow types in `__private_api`)

Opaque (already `#[hax_lib::opaque]` in source):
- `potentially_fix_state_incorrectly_encoded_by_libcrux_issue_1275` -- contains `log::info!`/`log::warn!` via `#[cfg(not(hax))]`
- `flip_endianness_of_encapsulation_state` -- helper for above

Aeneas reports 2 cosmetic errors (log macro internals) but output is clean: 0 sorry, 139 transparent functions.

## Modules not yet extracted (Charon hangs)

Charon hangs at MIR analysis level due to deep generic trait hierarchies in external crypto crates. Config flags (`--opaque`, `--exclude`, `--start-from`) have no effect since the hang occurs before they are applied. Upstream Charon issue.

- `spqr::kdf` -- Charon hangs (hkdf, sha2)
- `spqr::chain` -- Charon hangs (depends on kdf)
- `spqr::authenticator` -- Charon hangs (libcrux_hmac)
- `spqr::v1` -- Charon hangs (depends on authenticator, kdf, incremental_mlkem768)
- `spqr::proto` -- Charon hangs (prost-generated code)
