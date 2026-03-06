# UVL-Driven Variant Generation in Rust

This project provides a Rust macro crate that enables 
compile-time variant generation based on configurations created from 
UVL (Universal Variability Language) feature models.
The idea is to use concrete UVL configurations generated with UVLS
and integrate them directly into the Rust compilation process. 
Instead of using a traditional textual preprocessor, variability is 
implemented using Rust's macro system.

This repository contains:

- `uvl_macros` — the macro crate
- `example_car` — a small example project demonstrating how the macros are used
- `feature_model` — an example UVL feature model and a generated configuration file


---

## Input: UVL Configurations

The macros expect a JSON configuration for a UVL feature model. 
Specifically, a configuration created by **UVLS (Universal Variability Language Server)**: 
https://github.com/Universal-Variability-Language/uvls

You can also create the JSONs yourself, 
but then you must use the format of UVLS configurations.
The configuration file must be available during compilation.


---

## How It Works

During compilation, the macros:

1. Load the JSON configuration file.
2. Parse and evaluate the given feature expression.
3. Generate Rust code depending on the evaluation result.
4. Replace the macro invocation with the desired code.

If an expression evaluates to `false`, the corresponding code is
removed before the compiler processes the program.


---

## Provided Macros

The crate provides four macros for implementing
variability at compile time.


### `feat_if!`

```rust
feat_if!("FeatureExpression", {
    // code if expression is true
} else {
    // optional: code if expression is false
});
```


### `feat_ifdef!`

```rust
feat_ifdef!("FeatureName", {
    // code if feature is selected
} else {
    // optional: code if feature is not selected
});
```


### `feat_value!`

```rust
let value = feat_value!("FeatureName");
```


### `#[feat("...")]`

```rust
#[feat("FeatureExpression")]
fn some_function() {
    // only compiled if expression is true
}
```


### Helper Function: `sel(...)`

The function can be used within feature expressions to map features to numeric values.

```rust
sel(FeatureName)
```

The conversion rules are:
- `true` &rarr; `1`
- `false` &rarr; `0`
- integers &rarr; unchanged
- floats &rarr; rounded to an integer
- non-empty strings &rarr; `1`
- empty strings &rarr; `0`


---

## Examples

```rust
let numeric_value = feat_value!("FeatureNumericValue");
let string_value  = feat_value!("FeatureStringValue");

feat_if!("FeatureA", {
    println!("FeatureA is enabled.");
} else {
    println!("FeatureA is disabled.");
});

feat_if!("FeatureA && !FeatureB", {
    println!("FeatureA selected and FeatureB unselected.");
});

feat_if!("FeatureNumericValue > 10", {
    println!("FeatureNumericValue is greater than 10.");
});

feat_if!("sel(FeatureA) + sel(FeatureB) <= 1", {
    println!("At most one feature is selected.");
});
```
