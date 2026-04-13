# Star Sight Calculation Fix - Correct Implementation

## Summary

Fixed star sight calculations based on user's correct manual example. The previous "fix" (commit fce2475) was **WRONG** and has been reverted.

## The User's Correct Example (Pollux, 10 Sept 2016, 00:28:12 UTC)

**Critical insights from user:**
1. GHA Pollux = 78° 57.6' + correction 7° 05.7' = **86° 03.3'** (this is GHA of the star, already GHA Aries + SHA combined)
2. DR Longitude ≈ 20° W
3. **Optimize to make LHA of the STAR whole** (not LHA Aries!)
4. Chosen Longitude = 20° 03.9' W (to make LHA Pollux = 66° exactly)
5. LHA = GHA - Longitude_West = 86° 03.3' - 20° 03.9' = **66°** (exactly whole)
6. Using Pub 249 Vol 1 with LHA = 66°: **Hc = 46° 04', Az = 104° T** ✓

## The Problem with Commit fce2475 (WRONG!)

The commit fce2475 changed the code to:
- Optimize based on **GHA Aries** instead of GHA of the star
- Make **LHA Aries** whole instead of LHA of the star
- This gave **WRONG** results for star sights

**Why it was wrong:**
```
If we optimize LHA Aries instead of LHA star:
- GHA Pollux = 86.055°
- SHA Pollux ≈ 243.4°
- GHA Aries = 86.055° - 243.4° = 202.655°
- If we make LHA Aries whole → chosen lon = 19.655° W
- Then LHA Pollux = 66.4° (NOT whole!)
- This gives WRONG Az and Hc values
```

## The Correct Solution

**For ALL celestial bodies (including stars):**
1. Use **GHA of the body** (for stars, this is GHA Aries + SHA, already combined in almanac)
2. Optimize to make **LHA of the body** whole
3. Use `optimize_chosen_position(dr_lat, dr_lon, gha_body)` for everything

**Formula:**
```
LHA = GHA + Longitude (where East is +, West is -)
```

**For West longitude:**
```
LHA = GHA + (-Longitude) = GHA - Longitude_West
```

## Changes Made

### 1. `/Users/alex/work/celtui/celtnav/src/sight_reduction.rs`

**Updated `optimize_chosen_position()` function:**
- Enhanced documentation to clarify it works for ALL bodies including stars
- For stars, pass GHA of the star (GHA Aries + SHA combined)
- Optimizes to make LHA of the body whole
- Inlined the implementation (removed redirect to `optimize_chosen_position_celestial_body`)

**Deprecated `optimize_chosen_position_star()` function:**
- Marked with `#[deprecated]` attribute
- Updated documentation to explain why it's wrong
- Included user's example proving it's incorrect
- Redirects to `optimize_chosen_position_celestial_body` for backward compatibility

**Removed incorrect tests:**
- `test_optimize_star_chosen_position_lha_aries_whole`
- `test_optimize_star_vs_celestial_body_different_results`
- `test_pub249_vol1_organization`
- `test_star_optimization_multiple_scenarios`
- `test_star_optimization_edge_cases`
- `test_pollux_example_star_vs_body_optimization`

### 2. `/Users/alex/work/celtui/celtui/src/auto_compute_screen.rs`

**Updated imports:**
- Removed `optimize_chosen_position_celestial_body` and `optimize_chosen_position_star`
- Removed `gha_aries` import
- Use only `optimize_chosen_position`

**Simplified chosen position optimization:**
```rust
// OLD (WRONG):
let (chosen_lat, chosen_lon) = if sight.is_star() {
    let gha_aries_val = gha_aries(datetime);
    optimize_chosen_position_star(dr_latitude, dr_longitude, gha_aries_val)
} else {
    optimize_chosen_position_celestial_body(dr_latitude, dr_longitude, position.gha)
};

// NEW (CORRECT):
let (chosen_lat, chosen_lon) = optimize_chosen_position(dr_latitude, dr_longitude, position.gha);
```

**Simplified LHA calculation:**
```rust
// OLD (WRONG):
let (lha, gha_aries_val, lha_aries_val) = if sight.is_star() {
    let gha_aries_val = gha_aries(datetime);
    let lha_aries = (gha_aries_val + chosen_lon + 360.0) % 360.0;
    let lha_star = (position.gha + chosen_lon + 360.0) % 360.0;
    (lha_star, Some(gha_aries_val), Some(lha_aries))
} else {
    let lha = (position.gha + chosen_lon + 360.0) % 360.0;
    (lha, None, None)
};

// NEW (CORRECT):
let lha = (position.gha + chosen_lon + 360.0) % 360.0;
```

**Updated `LopDisplayData` structure:**
- Removed `gha_aries: Option<f64>` field
- Removed `lha_aries: Option<f64>` field
- Simplified to only show GHA and LHA of the body
- Updated documentation

### 3. `/Users/alex/work/celtui/celtui/src/export.rs`

- Removed test code references to `gha_aries` and `lha_aries`

### 4. Added New Test File

**`/Users/alex/work/celtui/celtnav/tests/pollux_user_example_test.rs`:**
- `test_pollux_user_example_optimization`: Verifies user's Pollux example
- `test_lha_formula_west_longitude`: Tests LHA formula for West longitude
- `test_pollux_lha_aries_vs_lha_pollux`: Demonstrates why optimizing LHA Aries is wrong

Output from test:
```
GHA Pollux: 86.055°
SHA Pollux: 243.400°
GHA Aries: 202.655°

CORRECT method (optimize LHA Pollux):
  Chosen Lon: 20.055° W
  LHA Pollux: 66.000°

WRONG method (optimize LHA Aries):
  Chosen Lon: 19.655° W
  LHA Pollux: 66.400°
```

### 5. Removed Incorrect Files

- `celtnav/tests/star_optimization_tests.rs` (all tests were wrong)
- `STAR_SIGHT_FIX_SUMMARY.md` (documented the wrong fix)

## Why the Confusion About Pub 249 Vol 1?

**Pub 249 Vol 1 Organization:** Tables are indexed by:
- Latitude (whole degrees)
- **LHA Aries** (whole degrees)
- Star name

**BUT:** This is just how the tables are **organized**, not how the **calculation** works!

**The calculation still uses:**
- LHA of the star (not LHA Aries)
- The formula: `sin(Hc) = sin(Lat) × sin(Dec) + cos(Lat) × cos(Dec) × cos(LHA_star)`

**The almanac gives you:**
- GHA Aries at the time
- SHA of the star (constant)
- **GHA star = GHA Aries + SHA** (this is what we use!)

**For sight reduction:**
1. Look up GHA Aries and SHA from almanac
2. Calculate GHA star = GHA Aries + SHA
3. Optimize longitude to make **LHA star** whole
4. Use LHA star in the spherical trig formula
5. For Pub 249 Vol 1, you can back-calculate LHA Aries if needed for table lookup

## Test Results

All tests passing:
- 104 unit tests in celtnav
- 103 integration tests
- 265 celtui tests
- 18 doctests
- **Total: 490 tests passing**

New tests added:
- `test_pollux_user_example_optimization` ✓
- `test_lha_formula_west_longitude` ✓
- `test_pollux_lha_aries_vs_lha_pollux` ✓

## Verification

The fix has been verified against:
1. User's manual calculation (Pollux example) ✓
2. Standard celestial navigation formulas ✓
3. Sight reduction principles ✓
4. All existing tests pass ✓

## Key Takeaways

1. **Always use GHA of the body** (for stars, this is GHA Aries + SHA combined)
2. **Optimize to make LHA of the body whole** (not LHA Aries for stars)
3. **LHA formula with signed convention:** LHA = GHA + Longitude (East +, West -)
4. **The spherical trig uses LHA of the body**, regardless of how tables are organized
5. **Trust the user's manual calculation** when it gives correct results!

## Related Files

Modified:
- `/Users/alex/work/celtui/celtnav/src/sight_reduction.rs`
- `/Users/alex/work/celtui/celtui/src/auto_compute_screen.rs`
- `/Users/alex/work/celtui/celtui/src/export.rs`

Added:
- `/Users/alex/work/celtui/celtnav/tests/pollux_user_example_test.rs`
- `/Users/alex/work/celtui/STAR_SIGHT_CORRECT_FIX.md` (this file)

Removed:
- `celtnav/tests/star_optimization_tests.rs`
- `STAR_SIGHT_FIX_SUMMARY.md`
