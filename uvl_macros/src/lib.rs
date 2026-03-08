// To define custom procedural macros
use proc_macro::TokenStream;

// Enables the generation of Rust code within procedural macros.
use quote::quote;

// For input parsing (token stream)
use syn::{parse_macro_input, LitStr, Block};
use syn::parse::{Parse, ParseStream};
use syn::Token;

// To read files
use std::fs;

// To read environment variable in config.toml
use std::env;

// To evaluate the presence conditions 
use evalexpr::*;

// To work with file paths
use std::path::PathBuf;

// To collect the feature names from the feature expression (presence condition)
use std::collections::HashSet;



// TODO: ThisError for Error Messages



// To get the path and file name of the configuration json
fn get_path_for_config_json() -> (String, PathBuf) {

    // Folder of the currently executed project
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("ERROR: CARGO_MANIFEST_DIR not found!");
    
    // Create Cargo.toml path
    let base_path = PathBuf::from(manifest_dir);
    let cargo_toml_path = base_path.join("Cargo.toml");
    
    // Read Cargo.toml
    let cargo_content = match fs::read_to_string(&cargo_toml_path) {
        Ok(content) => content,
        Err(e) => panic!(
            "CARGO READ ERROR: Could not read 'Cargo.toml'.\n\
            Searched at path: {}\n\
            Error: {}", 
            cargo_toml_path.display(), e
        ),
    };

    // Check whether it is a valid toml file
    let cargo_toml: toml::Value = match toml::from_str(&cargo_content) {
        Ok(toml) => toml,
        Err(e) => panic!(
            "CARGO SYNTAX ERROR: Your 'Cargo.toml' contains invalid toml.\n\
            Searched at path: {}\n\
            Error: {}", 
            cargo_toml_path.display(), e
        ),
    };

    let config_filename_ref = get_config_path_from_cargotoml(&cargo_toml);
    let config_filename = config_filename_ref.to_string();

    // Create path of configuration json file
    let config_path = base_path.join(&config_filename);


    (config_filename, config_path)

}



// Search for configuration json path in [package.metadata] in Cargo.toml
fn get_config_path_from_cargotoml(cargo_toml: &toml::Value) -> &str {
    
    // Search for package section in toml file
    let package_section = match cargo_toml.get("package") {
        Some(value) => value,
        None => panic!("CARGO TOML ERROR: '[package]' section is missing in Cargo.toml!"),
    };

    // Search for metadata section
    let metadata_section = match package_section.get("metadata") {
        Some(value) => value,
        None => panic!("CARGO TOML ERROR: '[package.metadata]' section is missing in Cargo.toml!\n\
            Please add this to your Cargo.toml:\n\n\
            [package.metadata]\n\
            uvl_config_path = \"config.uvl.json\"\n\n\
            Note: \"config.uvl.json\" must be the path of your configuration json!"
        ),
    };

    // Search for uvl_config_path value in metadata
    let uvl_config_path_value = match metadata_section.get("uvl_config_path") {
        Some(value) => value,
        None => panic!(
            "CARGO TOML ERROR: The configuration path is missing in Cargo.toml!\n\
            Please add this to your Cargo.toml:\n\n\
            [package.metadata]\n\
            uvl_config_path = \"config.uvl.json\"\n\n\
            Note: \"config.uvl.json\" must be the path of your configuration json!"
        ),
    };

    // Check if configuration path in [package.metadata] in Cargo.toml is a string
    let config_filename = match uvl_config_path_value.as_str() {  // as_str() to get json path without quotation marks
        Some(path) => path,
        None => panic!(
            "CARGO TOML ERROR: The 'uvl_config_path' value in [package.metadata] \
            in your Cargo.toml must be a string!"),
    };


    config_filename

}



// Replace spaces and hyphens in feature names
fn normalize_feature_name(s: &str) -> String {
    s.replace('"', "")
     .replace(' ', "_")
     .replace('-', "_")
}



// Loads the features from the configuration json that are required to evaluate the feature expressions
fn load_config(required_features: HashSet<String>) -> (HashMapContext<DefaultNumericTypes>, PathBuf) {
    // HashMapContext is part of evalexpr
    // It stores the enum Value, enabling different data types to be stored in a hash map.
    
    let (config_filename, config_path) = get_path_for_config_json();

    // Read UVL configuration json
    let uvl_config_content = match fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => {
            panic!(
                "Failed to read the UVL configuration file!\n\
                uvl_config_path is: '{}'\n\
                Resolved absolute path: {}\n\
                Tip: Check [package.metadata] uvl_config_path in your Cargo.toml!", 
                config_filename, config_path.display()
            );
        }
    };

    // Parsing UVL json configuration
    let json_val: serde_json::Value = match serde_json::from_str(&uvl_config_content) {
        Ok(v) => v,
        Err(e) => panic!("JSON PARSING ERROR: Your File '{}' is not a valid json: {}", config_path.display(), e),
    };

    // Read the config field from the json
    let config_map = match json_val.get("config").and_then(|v| v.as_object()) {
        Some(map) => map,
        None => panic!("JSON FORMAT ERROR: The json must contain a 'config' field."),
    };

    // Fill context object. This Maps names (strings) to values or functions (evalexpr::Value or evalexpr::Function).
    let mut feature_config = HashMapContext::<DefaultNumericTypes>::new();
    

    // Add a function that can convert a boolean feature to an int
    feature_config.set_function("sel".into(), Function::new(|argument| {
        match argument {
            // If it is a boolean: true --> 1, false --> 0
            Value::Boolean(b)    => Ok(Value::Int(if *b { 1 } else { 0 })), 
            // If already a int, do nothing
            Value::Int(i)         => Ok(Value::Int(*i)),
            // If it is a float, then it is rounded
            Value::Float(f)       => Ok(Value::Int((*f as f64).round() as i64)),
            // An empty string is mapped to 0, otherwise to 1
            Value::String(s)   => Ok(Value::Int(if s.is_empty() { 0 } else { 1 })),

            // Every other type is an error
            _ => Err(evalexpr::EvalexprError::CustomMessage(
                format!("TYPE ERROR: Function 'sel()' expects Boolean, Int, Float or String but got {:?}", argument)
            )),
        }
    })).expect("FUNCTION ERROR: Failed to set sel()-function!");


    for (key, val) in config_map {

        let clean_key = normalize_feature_name(key);

        // Only features, which are in the expression, are considered
        if !required_features.contains(&clean_key) {
            continue;
        }

        // --- Feature type matching ---
        // Feature Cardinality (array in json):
        if let Some(card_array) = val.as_array() {  // if option enum is Some, then it is an array.
            
            let mut count = 0;
            
            // Count the number of objects in the array that are true
            for item in card_array {
                if let Some(obj) = item.as_object() {
                    if let Some(first_value) = obj.values().next() {
                        // Check if this value is true
                        if first_value.as_bool() == Some(true) {
                            count += 1;
                        }
                    }
                }
            }

            feature_config.set_value(clean_key.into(), Value::Int(count))
                .expect("Failed to store cardinality value in HashMapContext!");
        } 
        // Integer Feature (Int):
        else if let Some(int_val) = val.as_i64() {
            feature_config.set_value(clean_key.into(), Value::Int(int_val))
                .expect("Failed to store int value in HashMapContext!");
        }
        // Real Feature (Float):
        else if let Some(num) = val.as_f64() {
            feature_config.set_value(clean_key.into(), Value::from_float(num))
                .expect("Failed to store float value in HashMapContext!");
        } 
        // Boolean Feature:
        else if let Some(bool_val) = val.as_bool() {
            feature_config.set_value(clean_key.into(), Value::from(bool_val))
                .expect("Failed to store boolean value in HashMapContext!");
        } 
        // String Feature:
        else if let Some(str_val) = val.as_str() {
            feature_config.set_value(clean_key.into(), Value::from(str_val.to_string()))
                .expect("Failed to store string value in HashMapContext!");
        }
        // Unsupported Type in json:
        else {
            panic!("FEATURE TYPE ERROR: The feature '{}' has an unsupported type in the JSON.", clean_key);
        }
    }


    (feature_config, config_path)

}



// Struct for token steam parsing for uvl_if!("Condition", { ... } else {...})
// Macros work with token stream: Crate syn provide data structure for Rust syntax
struct FeatureIfInput {
    // Literal String for Condition --> Presence Condition for block
    condition:      LitStr,     

    // Comma Token (Singleton-Type) --> necessary for parsing but is not used: (_)
    _comma:         Token![,],

    // Rust code block that is either kept or discarded.
    // A block can be inserted directly back into the program with quote! as if it were normal code.
    code_block:     Block,    

    // Else case for condition with else token and else code block
    else_branch:    Option<(Token![else], Block)>,  
}


// Defines Trait Parse for FeatureIfInput struct
impl Parse for FeatureIfInput {
    // Parse the macro input
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Rusts Type Inference: checks the type of the variable of the left side
        let condition: LitStr = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let code_block: Block = input.parse()?;
        
        // Check if the next token is an else
        let else_branch = if input.peek(Token![else]) {
            let else_token: Token![else] = input.parse()?;
            let else_block: Block = input.parse()?;
            Some((else_token, else_block))
        } else {
            // No else block
            None
        };

        Ok(FeatureIfInput {
            condition,
            _comma,
            code_block,
            else_branch,
        })
    }
}



// ================================================================================
// ==================================== MACROS ====================================
// ================================================================================

/*
NOTE FOR USAGE:
1) If a feature name consists of more than one word, the spaces must be replaced with underscores!
2) Undefined features are mapped to false.

LIMITATIONS:
1) There is no sum, avg, length function like in UVL.
2) It is currently not possible to distinguish between feature clones.
3) Feature Groups in clones and clones in clones are not supported yet. 
4) There is currently no function like defined(), just the macro feat_ifdef!.
*/


// --- If Macro ---
// To define a presence condition for a code block
#[proc_macro]
pub fn feat_if(input: TokenStream) -> TokenStream {
    // The Parse Trait must be defined for this macro
    let input   = parse_macro_input!(input as FeatureIfInput); 
    let condition_str   = input.condition.value();
    let code_block       = input.code_block;

    // Save required feature names
    let mut required_features = HashSet::new();

    // Extract operator tree to get the identifiers (feature names from the expression)
    if let Ok(expr) = build_operator_tree::<DefaultNumericTypes>(&condition_str) {
        // Get feature names from expression
        for variable in expr.iter_identifiers() {
            // Add feature names to the required set of features
            required_features.insert(variable.to_string());
        }
    }
    
    // Load required features from configuration json
    let (mut feature_config, config_path) = load_config(required_features.clone());
    let config_path_str = config_path.to_string_lossy().to_string();

    // If a feature is missing in the json, it is set to false
    for req_feat in required_features {
        let clean_key = normalize_feature_name(&req_feat);
        if feature_config.get_value(&clean_key).is_none() {
            // Add missing feature in configuration as false
            feature_config.set_value(clean_key.into(), Value::Boolean(false))
                .expect("Failed to store a missing feature as boolean value (false) in HashMapContext!");
        }
    }

    // Evaluate expression with Crate evalexpr
    let eval_result = eval_with_context(&condition_str, &feature_config);

    // Evaluate the result of the condition
    let is_condition_true = match eval_result {
        // If evaluation returns a number, then it has to be converted into a boolean value
        Ok(value) => match value {
            // Value is already boolean
            Value::Boolean(b) => b,

            // Value is an int, need to generate a boolean expression
            Value::Int(i) => i != 0,

            // Value is a float, need to generate a boolean expression
            Value::Float(f) => f != 0.0,

            // Unsupported data type
            _ => panic!("EXPRESSION EVALUATION ERROR: Condition '{}' evaluated to \
                        an unsupported type.", condition_str),
        },
        Err(e) => panic!("EXPRESSION EVALUATION ERROR: Failed to \
                        evaluate '{}': {}", condition_str, e),
    };

    if is_condition_true {
        // # is necessary so that the actual code stored in the block is inserted
        // Insert code block
        quote! { 
            // In order to use the macro in expressions, parentheses must be placed around the code block. 
            {
                // Trick: This means that the program always has to check the json again. 
                // Previously, you always had to save the json and the project again so that the macros would be updated.
                // https://users.rust-lang.org/t/logging-file-dependency-like-include-bytes-in-custom-macro/57441/2
                const _: &[u8] = include_bytes!(#config_path_str);
                
                #[allow(unused_braces)]
                let result = #code_block;
                
                result 
            }
            
        }.into()  // .into() converts the quote tokenstream to the standard tokenstream 
    } else {
        // Check if there is an else block
        if let Some((_else_token, else_block)) = input.else_branch {
            // Insert else code block
            quote! { 
                // In order to use the macro in expressions, parentheses must be placed around the code block. 
                // To avoid warnings about too many parentheses, this is allowed here.
                {
                    const _: &[u8] = include_bytes!(#config_path_str);

                    #[allow(unused_braces)]
                    let result = #else_block;
                    
                    result 
                }
            }.into()
        } else {
            // There is no else block, insert empty code block
            quote! { 
                const _: &[u8] = include_bytes!(#config_path_str);
                {} 
            }.into() 
        }
    }
}


// --- IfDefined Macro ---
// Check whether the feature is defined, i.e., whether the feature is included in the configuration json
#[proc_macro]
pub fn feat_ifdef(input: TokenStream) -> TokenStream {
    // The Parse Trait must be defined for this macro
    let input      = parse_macro_input!(input as FeatureIfInput); 
    let raw_feature_name   = input.condition.value(); 
    let code_block          = input.code_block;

    let feature_name = normalize_feature_name(&raw_feature_name);
    
    // Save required feature names
    let mut required_features = HashSet::new();

    // Add feature name to the required set of features
    required_features.insert(feature_name.clone());
    
    // Load required features from configuration json
    let (feature_config, config_path) = load_config(required_features);
    let config_path_str = config_path.to_string_lossy().to_string();

    // Check if the feature exists in the configuration json
    // get_value().is_some() is true if the key was found and successfully parsed
    let is_feature_defined = feature_config.get_value(&feature_name).is_some();

    if is_feature_defined {
        // # is necessary so that the actual code stored in the block is inserted
        // Insert code block
        quote! { 
            // In order to use the macro in expressions, parentheses must be placed around the code block. 
            // To avoid warnings about too many parentheses, this is allowed here.
            {
                // Trick: This means that the program always has to check the json again. 
                // Previously, you always had to save the json and the project again so that the macros would be updated.
                // https://users.rust-lang.org/t/logging-file-dependency-like-include-bytes-in-custom-macro/57441/2
                const _: &[u8] = include_bytes!(#config_path_str);
                
                #[allow(unused_braces)]
                let result = #code_block;
                
                result
            }
        }.into()
    } else {
        if let Some((_else_token, else_block)) = input.else_branch {
            // Insert else code block
            quote! { 
                // In order to use the macro in expressions, parentheses must be placed around the code block. 
                // To avoid warnings about too many parentheses, this is allowed here.
                {
                    const _: &[u8] = include_bytes!(#config_path_str);

                    #[allow(unused_braces)]
                    let result = #else_block;
                    
                    result 
                }
            }.into()
        } else {
            // Insert empty code block
            quote! { 
                // In order to use the macro in expressions, parentheses must be placed around the code block. 
                {
                    const _: &[u8] = include_bytes!(#config_path_str);
                    {}
                } 
            }.into() 
        }
    }
}



// --- Value Macro ---
// To get the value of a feature
#[proc_macro]
pub fn feat_value(input: TokenStream) -> TokenStream {
    let key_str_lit      = parse_macro_input!(input as LitStr);  // Parse feature name to Literal String
    let raw_feature_name = key_str_lit.value();  // Convert Literal String to String

    let feature_name = normalize_feature_name(&raw_feature_name);

    // Save required feature names
    let mut required_features = HashSet::new();

    // Add feature name to the required set of features
    required_features.insert(feature_name.clone());
    
    // Load json configuration
    let (feature_config, config_path) = load_config(required_features);
    let config_path_str = config_path.to_string_lossy().to_string();
    
    // Get value from feature in hashmap
    let feature_value = feature_config.get_value(&feature_name);

    let val = match feature_value {
        Some(v) => v,
        None => panic!("CONFIGURATION ERROR: Feature '{}' could not be found in json!", feature_name),
    };

    // Insert feature value based on type
    let val_as_tokenstream = match val {
        // # is necessary so that the actual value stored in the feature is inserted
        Value::Float(f)     => quote! { #f },
        Value::Int(i)       => quote! { #i },
        Value::Boolean(b)  => quote! { #b },
        Value::String(s) => quote! { #s },
        _ => panic!("INSERT FEATURE VALUE ERROR: The type of '{}' is not supported!", feature_name),
    };

    let val_as_tokenstream_trick = quote! {
        {
            // Trick: This means that the program always has to check the json again. 
            // Previously, you always had to save the json and the project again so that the macros would be updated.
            // https://users.rust-lang.org/t/logging-file-dependency-like-include-bytes-in-custom-macro/57441/2
            const _: &[u8] = include_bytes!(#config_path_str);
            #val_as_tokenstream
        }
    };

    val_as_tokenstream_trick.into()
}



// --- Attribute Macro ---
// To remove items (functions, structs, fields, etc.) based on a condition.
// Usage: #[feat("FeatureName")] or #[feat("A && B")]
#[proc_macro_attribute]
pub fn feat(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the condition string)
    let condition_lit = parse_macro_input!(attr as LitStr);
    let condition_str = condition_lit.value();

    // Save required feature names
    let mut required_features = HashSet::new();

    // Extract operator tree to get the identifiers (feature names from the expression)
    if let Ok(expr) = build_operator_tree::<DefaultNumericTypes>(&condition_str) {
        // Get feature names from expression
        for variable in expr.iter_identifiers() {
            // Add feature names to the required set
            required_features.insert(variable.to_string());
        }
    }

    // Load required features from configuration json
    let (mut feature_config, _config_path) = load_config(required_features.clone());

    // If a feature is missing in the json, it is set to false
    for req_feat in required_features {
        let clean_key = normalize_feature_name(&req_feat);
        if feature_config.get_value(&clean_key).is_none() {
            // Add missing feature in configuration as false
            feature_config.set_value(clean_key.into(), Value::Boolean(false))
                .expect("Failed to store a missing feature as boolean value (false) in HashMapContext!");
        }
    }

    // Evaluate expression with Crate evalexpr
    let eval_result = eval_with_context(&condition_str, &feature_config);

    // Evaluate the result of the condition
    let is_condition_true = match eval_result {
        // If evaluation returns a number, then it has to be converted into a boolean value
        Ok(value) => match value {
            // Value is already boolean
            Value::Boolean(b) => b,

            // Value is an int, need to generate a boolean expression
            Value::Int(i) => i != 0,

            // Value is a float, need to generate a boolean expression
            Value::Float(f) => f != 0.0,

            // Unsupported data type
            _ => panic!("EXPRESSION EVALUATION ERROR: Condition '{}' evaluated to \
                        an unsupported type.", condition_str),
        },
        Err(e) => panic!("EXPRESSION EVALUATION ERROR: Failed to \
                        evaluate '{}': {}", condition_str, e),
    };

    if is_condition_true {
        // Keep item unchanged in the code)
        item
    } else {
        // Return empty TokenStream to remove the item from the code
        TokenStream::new()
    }
}
