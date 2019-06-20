//!  Macros for hosting embedded Rust applications on Mynewt

///  Return a const struct that has all fields set to 0. Used for initialising static mutable structs like `os_task`.
///  `fill_zero!(os_task)` expands to
///  ```
/// unsafe { 
///	::core::mem::transmute::
///	<
///	  [
///		u8; 
///		::core::mem::size_of::<os_task>()
///	  ], 
///	  os_task
///	>
///	(
///	  [
///		0; 
///		::core::mem::size_of::<os_task>()
///	  ]
///	) 
/// }
///  ```
#[macro_export]
macro_rules! fill_zero {
  ($type:ident) => {
    unsafe { 
        ::core::mem::transmute::
        <
        [
            u8; 
            ::core::mem::size_of::<$type>()
        ], 
        $type
        >
        (
        [
            0; 
            ::core::mem::size_of::<$type>()
        ]
        ) 
    }      
  };
}

///  Macro to compose a CoAP payloads with JSON or CBOR encoding.
///  Adapted from the `json!()` macro: https://docs.serde.rs/src/serde_json/macros.rs.html
#[macro_export(local_inner_macros)]
macro_rules! coap {
  //  No encoding
  (@none $($tokens:tt)+) => {
    parse!(@none $($tokens)+)
  };
  //  JSON encoding
  (@json $($tokens:tt)+) => {
    parse!(@json $($tokens)+)
  };
  //  CBOR encoding
  (@cbor $($tokens:tt)+) => {
    parse!(@cbor $($tokens)+)
  };
}

#[macro_export(local_inner_macros)]
macro_rules! parse {

  //////////////////////////////////////////////////////////////////////////
  // TT muncher for parsing the inside of an object {...}. Each entry is
  // inserted into the given map variable.
  //
  // Must be invoked as: parse!(@$enc @object $map () ($($tt)*) ($($tt)*))
  //
  // We require two copies of the input tokens so that we can match on one
  // copy and trigger errors on the other copy.
  //////////////////////////////////////////////////////////////////////////

  // Done.
  (@$enc:ident @object $object:ident () () ()) => {};

  // No Encoding: Insert the current entry followed by trailing comma.
  (@none @object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
    d!(TODO: add key: $($key)+, value: $value, to object: $object);

    //  Previously:
    //  let _ = $object.insert(($($key)+).into(), $value);

    //  Continue expanding the rest of the JSON.
    parse!(@none @object $object () ($($rest)*) ($($rest)*));
  };

  // JSON and CBOR Encoding: Insert the current entry followed by trailing comma.
  (@$enc:ident @object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
    d!(add1 key: $($key)+ value: $value to object: $object);

    //  Append to the "values" array e.g.
    //    {"key":"device", "value":"0102030405060708090a0b0c0d0e0f10"},
    coap_item_str!(@$enc $object, $($key)+, $value);
    "--------------------";

    //  Previously:
    //  let _ = $object.insert(($($key)+).into(), $value);

    //  Continue expanding the rest of the JSON.
    parse!(@$enc @object $object () ($($rest)*) ($($rest)*));
  };

  // Current entry followed by unexpected token.
  (@$enc:ident @object $object:ident [$($key:tt)+] ($value:expr) $unexpected:tt $($rest:tt)*) => {
    unexpected_token!($unexpected);
  };

  // Insert the last entry without trailing comma.
  (@$enc:ident @object $object:ident [$($key:tt)+] ($value:expr)) => {
    //  TODO
    d!(TODO: add2 key: $($key)+ value: $value to object: $object);
    //  let _ = $object.insert(($($key)+).into(), $value);
  };

  // Next value is `null`.
  (@$enc:ident @object $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
    parse!(@$enc @object $object [$($key)+] (parse!(@$enc null)) $($rest)*);
  };

  // Next value is `true`.
  (@$enc:ident @object $object:ident ($($key:tt)+) (: true $($rest:tt)*) $copy:tt) => {
    parse!(@$enc @object $object [$($key)+] (parse!(@$enc true)) $($rest)*);
  };

  // Next value is `false`.
  (@$enc:ident @object $object:ident ($($key:tt)+) (: false $($rest:tt)*) $copy:tt) => {
    parse!(@$enc @object $object [$($key)+] (parse!(@$enc false)) $($rest)*);
  };

  // Next value is an array.
  (@$enc:ident @object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
    parse!(@$enc @object $object [$($key)+] (parse!(@$enc [$($array)*])) $($rest)*);
  };

  // Next value is a map.
  (@$enc:ident @object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
    parse!(@$enc @object $object [$($key)+] (parse!(@$enc {$($map)*})) $($rest)*);
  };

  // Next value is an expression followed by comma.
  (@$enc:ident @object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
    parse!(@$enc @object $object [$($key)+] (parse!(@$enc $value)) , $($rest)*);
  };

  // Last value is an expression with no trailing comma.
  (@$enc:ident @object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
    parse!(@$enc @object $object [$($key)+] (parse!(@$enc $value)));
  };

  // Missing value for last entry. Trigger a reasonable error message.
  (@$enc:ident @object $object:ident ($($key:tt)+) (:) $copy:tt) => {
    // "unexpected end of macro invocation"
    parse!();
  };

  // Missing colon and value for last entry. Trigger a reasonable error
  // message.
  (@$enc:ident @object $object:ident ($($key:tt)+) () $copy:tt) => {
    // "unexpected end of macro invocation"
    parse!();
  };

  // Misplaced colon. Trigger a reasonable error message.
  (@$enc:ident @object $object:ident () (: $($rest:tt)*) ($colon:tt $($copy:tt)*)) => {
    // Takes no arguments so "no rules expected the token `:`".
    unexpected_token!($colon);
  };

  // No Encoding: Found a key followed by a comma. Assume this is a SensorValue type with key and value.
  (@none @object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
    d!(TODO: extract key, value from _sensor_value: $($key)* and add to _object: $object);
    "--------------------";
    //  Continue expanding the rest of the JSON.
    parse!(@none @object $object () ($($rest)*) ($($rest)*));
  };

  // JSON Encoding: Found a key followed by a comma. Assume this is a SensorValue type with key and value.
  (@json @object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
    d!(TODO: extract key, value from _sensor_value: $($key)* and add to _object: $object);
    "--------------------";
    coap_item_int_val!(@json
      $object,  //  _object, 
      $($key)*  //  _sensor_value
    );
    "--------------------";
    //  Continue expanding the rest of the JSON.
    parse!(@json @object $object () ($($rest)*) ($($rest)*));
  };

  // CBOR Encoding: Found a key followed by a comma. Assume this is a SensorValue type with key and value.
  (@cbor @object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
    d!(TODO: extract key, value from _sensor_value: $($key)* and add to _object: $object);
    "--------------------";
    coap_set_int_val!(@cbor
      $object,  //  _object, 
      $($key)*  //  _sensor_value
    );
    "--------------------";
    //  Continue expanding the rest of the JSON.
    parse!(@cbor @object $object () ($($rest)*) ($($rest)*));
  };

  // Previously: Found a comma inside a key. Trigger a reasonable error message.
  // Takes no arguments so "no rules expected the token `,`".
  ////unexpected_token!($comma);

  // Key is fully parenthesized. This avoids clippy double_parens false
  // positives because the parenthesization may be necessary here.
  (@$enc:ident @object $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
    d!( got () );
    parse!(@$enc @object $object ($key) (: $($rest)*) (: $($rest)*));
  };

  // Munch a token into the current key.
  (@$enc:ident @object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {    
    nx!( ($($key)*), ($tt), ($($rest)*) );
    //  Parse the next token while we are in the @object state.
    //  coap_internal takes these parameters:
    //  encoding: @json, @cbor or @none
    //  current state: @object ???
    //  current token: ???
    //  remaining tokens
    //  remaining tokens again, for error display
    parse!(@$enc @object $object ($($key)* $tt) ($($rest)*) ($($rest)*));
  };


  //////////////////////////////////////////////////////////////////////////
  // TT muncher for parsing the inside of an array [...]. Produces a vec![...]
  // of the elements.
  //
  // Must be invoked as: parse!(@$enc @array [] $($tt)*)
  //////////////////////////////////////////////////////////////////////////

  // Done with trailing comma.
  (@$enc:ident @array [$($elems:expr,)*]) => {
    parse_vector![$($elems,)*]
  };

  // Done without trailing comma.
  (@$enc:ident @array [$($elems:expr),*]) => {
    parse_vector![$($elems),*]
  };

  // Next element is `null`.
  (@$enc:ident @array [$($elems:expr,)*] null $($rest:tt)*) => {
    parse!(@$enc @array [$($elems,)* parse!(@$enc null)] $($rest)*)
  };

  // Next element is `true`.
  (@$enc:ident @array [$($elems:expr,)*] true $($rest:tt)*) => {
    parse!(@$enc @array [$($elems,)* parse!(@$enc true)] $($rest)*)
  };

  // Next element is `false`.
  (@$enc:ident @array [$($elems:expr,)*] false $($rest:tt)*) => {
    parse!(@$enc @array [$($elems,)* parse!(@$enc false)] $($rest)*)
  };

  // Next element is an array.
  (@$enc:ident @array [$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
    parse!(@$enc @array [$($elems,)* parse!(@$enc [$($array)*])] $($rest)*)
  };

  // Next element is a map.
  (@$enc:ident @array [$($elems:expr,)*] {$($map:tt)*} $($rest:tt)*) => {
    parse!(@$enc @array [$($elems,)* parse!(@$enc {$($map)*})] $($rest)*)
  };

  // Next element is an expression followed by comma.
  (@$enc:ident @array [$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
    parse!(@$enc @array [$($elems,)* parse!(@$enc $next),] $($rest)*)
  };

  // Last element is an expression with no trailing comma.
  (@$enc:ident @array [$($elems:expr,)*] $last:expr) => {
    parse!(@$enc @array [$($elems,)* parse!(@$enc $last)])
  };

  // Comma after the most recent element.
  (@$enc:ident @array [$($elems:expr),*] , $($rest:tt)*) => {
    parse!(@$enc @array [$($elems,)*] $($rest)*)
  };

  // Unexpected token after most recent element.
  (@$enc:ident @array [$($elems:expr),*] $unexpected:tt $($rest:tt)*) => {
    unexpected_token!($unexpected)
  };


  //////////////////////////////////////////////////////////////////////////
  // The main implementation.
  //
  // Must be invoked as: parse!(@$enc $($tokens)+) where $enc is json, cbor or none
  //////////////////////////////////////////////////////////////////////////

  (@$enc:ident null) => {{ 
    d!(TODO: null); "null" 
  }};  //  Previously: $crate::Value::Null

  (@$enc:ident true) => {
    //  TODO
    { d!(true); "true" }
    //  Previously:
    //  $crate::Value::Bool(true)
  };

  (@$enc:ident false) => {
    //  TODO
    { d!(false); "false" }
    //  Previously:
    //  $crate::Value::Bool(false)
  };

  (@$enc:ident []) => {
    //  TODO
    { d!([ TODO ]); "[ TODO ]" }
    //  Previously:
    //  $crate::Value::Array(parse_vector![])
  };

  (@$enc:ident [ $($tt:tt)+ ]) => {
    //  TODO
    {
      d!(begin array);
      _array = parse!(@$enc @array [] $($tt)+);
      d!(end array);
      "[ TODO ]"
    }
    //  Previously:
    //  $crate::Value::Array(parse!(@array [] $($tt)+))
  };

  (@$enc:ident {}) => {
    //  TODO
    { d!({ TODO }); "{ TODO }" }
    //  Previously:
    //  $crate::Value::Object($crate::Map::new())
  };

  //  No encoding: If we match the top level of the JSON: { ... }
  (@none { $($tt:tt)+ }) => {{
    //  Substitute with this code...
    d!(begin none root);
    let root = "root";  //  Top level object is named "root".
    //  Expand the items inside { ... } and add them to root.
    parse!(@none @object root () ($($tt)+) ($($tt)+));
    d!(end none root);
    d!(return none root to caller);
    root
  }};
  
  //  JSON encoding: If we match the top level of the JSON: { ... }
  (@json { $($tt:tt)+ }) => {{
    //  Substitute with this code...
    d!(begin json root);
    //  let root = "root";  //  Top level object is named "root".
    //  let values = "values";  //  "values" will be an array of items under the root
    let mut values_map: CborEncoder = fill_zero!(CborEncoder);
    let mut values_array: CborEncoder = fill_zero!(CborEncoder);
    coap_root!(@json {  //  Create the payload root
        coap_array!(@json root, values, {  //  Create "values" as an array of items under the root
          //  Expand the items inside { ... } and add them to values.
          parse!(@json @object values () ($($tt)+) ($($tt)+));
        });  //  Close the "values" array
    });  //  Close the payload root
    d!(end json root);
    d!(return json root to caller);
    root
  }};

  //  CBOR encoding: If we match the top level of the JSON: { ... }
  (@cbor { $($tt:tt)+ }) => {{
    //  Substitute with this code...
    d!(begin cbor root);
    let root = "root";  //  Top level object is named "root".
    coap_root!(@cbor {  //  Create the payload root
        //  Expand the items inside { ... } and add them to root.
        parse!(@cbor @object root () ($($tt)+) ($($tt)+));
    });  //  Close the payload root
    d!(end cbor root);
    d!(return cbor root to caller);
    root
  }};

  /* Previously substitute with:
  $crate::Value::Object({
    let mut object = $crate::Map::new();
    parse!(@object object () ($($tt)+) ($($tt)+));
    object
  })
  */

  // Any Serialize type: numbers, strings, struct literals, variables etc.
  // Must be below every other rule.
  (@$enc:ident $other:expr) => {
    //  Return itself.
    $other
  };  //  Previously: $crate::to_value(&$other).unwrap()
}

#[macro_export]
#[doc(hidden)]
macro_rules! parse_vector {
  ($($content:tt)*) => {
    vec![$($content)*]
  };
}

#[macro_export]
#[doc(hidden)]
macro_rules! unexpected_token {
  () => {};
}

///////////////////////////////////////////////////////////////////////////////
//  CoAP macros ported from C to Rust:
//  https://github.com/lupyuen/stm32bluepill-mynewt-sensor/blob/rust-coap/libs/sensor_coap/include/sensor_coap/sensor_coap.h

///  Compose the payload root.
#[macro_export(local_inner_macros)]
macro_rules! coap_root {  
  (@cbor $children0:block) => {{  //  CBOR
    d!(begin cbor coap_root);
    oc_rep_start_root_object!();
    $children0;
    oc_rep_end_root_object!();
    d!(end cbor coap_root);
  }};

  (@json $children0:block) => {{  //  JSON
    d!(begin json coap_root);
    unsafe { json_rep_start_root_object() }
    $children0;
    unsafe { json_rep_end_root_object() }
    d!(end json coap_root);
  }};
}

///  Compose an array under "object", named as "key".  Add "children" as array elements.
#[macro_export(local_inner_macros)]
macro_rules! coap_array {
  (@cbor $object0:ident, $key0:ident, $children0:block) => {{  //  CBOR
    d!(begin cbor coap_array, object: $object0, key: $key0);
    oc_rep_set_array!($object0, $key0);
    $children0;
    oc_rep_close_array!($object0, $key0);
    d!(end cbor coap_array);
  }};

  (@json $object0:ident, $key0:ident, $children0:block) => {{  //  JSON
    d!(begin json coap_array, object: $object0, key: $key0);
    json_rep_set_array!($object0, $key0);
    $children0;
    json_rep_close_array!($object0, $key0);
    d!(end json coap_array);
  }};
}

///  Append a (`key` + `val` string value) item to the array named `parent`:
///    `{ <parent>: [ ..., {"key": <key>, "value": <val>} ] }`
#[macro_export(local_inner_macros)]
macro_rules! coap_item_str {
  (@cbor $parent:ident, $key:expr, $val:expr) => {{  //  CBOR
    d!(begin cbor coap_item_str, parent: $parent, key: $key, val: $val);
    coap_item!(@cbor
      $parent,
      {
        oc_rep_set_text_string!($parent, "key", $key);
        oc_rep_set_text_string!($parent, "value", $val);
      }
    );
    d!(end cbor coap_item_str);
  }};

  (@json $parent:ident, $key:expr, $val:expr) => {{  //  JSON
    d!(begin json coap_item_str, parent: $parent, key: $key, val: $val);
    coap_item!(@json
      $parent,
      {
        json_rep_set_text_string!($parent, "key", $key);
        json_rep_set_text_string!($parent, "value", $val);
      }
    );
    d!(end json coap_item_str);
  }};
}

///  Append an array item under the array named `array0`.  Add `children0` as the items (key and value).
///    `{ <array0>: [ ..., { <children0> } ] }`
#[macro_export(local_inner_macros)]
macro_rules! coap_item {
  (@cbor $array0:ident, $children0:block) => {{  //  CBOR
    d!(begin cbor coap_item, array: $array0);
    oc_rep_object_array_start_item!($array0);
    $children0;
    oc_rep_object_array_end_item!($array0);
    d!(end cbor coap_item);
  }};

  (@json $array0:ident, $children0:block) => {{  //  JSON
    d!(begin json coap_item, array: $array0);
    json_rep_object_array_start_item!($array0);
    $children0;
    json_rep_object_array_end_item!($array0);
    d!(end json coap_item);
  }};
}

//  Append a (key + int value) item to the array named "array":
//    { <array>: [ ..., {"key": <key0>, "value": <value0>} ], ... }
#[macro_export(local_inner_macros)]
macro_rules! coap_item_int {
  (@cbor $array0:ident, $key0:expr, $value0:expr) => {{  //  CBOR
    d!(begin cbor coap_item_int, key: $key0, value: $value0);
    coap_item!(@$enc $array0, {
      oc_rep_set_text_string!($array0, "key",   $key0);
      oc_rep_set_int!(        $array0, "value", $value0);
    });
    d!(end cbor coap_item_int);
  }};

  (@json $array0:ident, $key0:expr, $value0:expr) => {{  //  JSON
    d!(begin json coap_item_int, key: $key0, value: $value0);
    coap_item!(@json $array0, {
      json_rep_set_text_string!($array0, "key",   $key0);
      json_rep_set_int!(        $array0, "value", $value0);
    });
    d!(end json coap_item_int);
  }};
}

///  Given an object parent and an integer Sensor Value val, set the val's key/value in the object.
#[macro_export(local_inner_macros)]
macro_rules! coap_set_int_val {
  (@cbor $parent0:ident, $val0:expr) => {{  //  CBOR
    d!(begin cbor coap_set_int_val, parent: $parent0, val: $val0);
    d!(> TODO: assert($val0.val_type == SENSOR_VALUE_TYPE_INT32));
    //  d!(> TODO: oc_rep_set_int_k($parent0, $val0.key, $val0.int_val));
    oc_rep_set_int!($parent0, $val0.key, 1234);  //  TODO
    d!(end cbor coap_set_int_val);
  }};

  (@json $parent0:ident, $val0:expr) => {{  //  JSON
    d!(begin json coap_set_int_val, parent: $parent0, val: $val0);
    d!(> TODO: assert($val0.val_type == SENSOR_VALUE_TYPE_INT32));
    //  d!(> TODO: oc_rep_set_int_k($parent0, $val0.key, $val0.int_val));
    json_rep_set_int!($parent0, $val0.key, 1234);  //  TODO
    d!(end json coap_set_int_val);
  }};
}

///  Create a new Item object in the parent array and set the Sensor Value's key/value (integer).
#[macro_export(local_inner_macros)]
macro_rules! coap_item_int_val {
  (@cbor $parent0:ident, $val0:expr) => {{  //  CBOR
    d!(begin cbor coap_item_int_val, parent: $parent0, val: $val0);
    d!(> TODO: assert($val0.val_type == SENSOR_VALUE_TYPE_INT32));
    d!(> TODO: coap_item_int(@cbor $parent0, $val0.key, $val0.int_val));
    coap_item_int!(@cbor $parent0, $val0.key, 1234);  //  TODO
    d!(end cbor coap_item_int_val);
  }};

  (@json $parent0:ident, $val0:expr) => {{  //  JSON
    d!(begin json coap_item_int_val, parent: $parent0, val: $val0);
    d!(> TODO: assert($val0.val_type == SENSOR_VALUE_TYPE_INT32));
    d!(> TODO: coap_item_int(@json $parent0, $val0.key, $val0.int_val));
    coap_item_int!(@json $parent0, $val0.key, 1234);  //  TODO
    d!(end json coap_item_int_val);
  }};
}

///////////////////////////////////////////////////////////////////////////////
//  JSON Sensor CoAP macros ported from C to Rust:
//  https://github.com/lupyuen/stm32bluepill-mynewt-sensor/blob/rust-coap/libs/sensor_coap/include/sensor_coap/sensor_coap.h

//  Assume we are writing an object now.  Write the key name and start a child array.
//  {a:b --> {a:b, key:[
#[macro_export]
macro_rules! json_rep_set_array {
  ($object:ident, $key:ident) => {{
    concat!(
      "begin json_rep_set_array ",
      ", object: ", stringify!($object),
      ", key: ",    stringify!($key),
      ", child: ",  stringify!($object), "_map"  //  object##_map
    );
    json_encode_array_name(&coap_json_encoder, #key); 
    json_encode_array_start(&coap_json_encoder);

    //  concat!("> TODO: g_err |= cbor_encode_text_string(&object##_map, #key, strlen(#key));");
    //  unsafe { cbor_encode_text_string(&mut concat_idents!($object, _map), $key.as_ptr(), $key.len()) };
    //  concat!("> TODO: json_rep_start_array!(object##_map, key);");

    d!(end json_rep_set_array);
  }};
}

//  End the child array and resume writing the parent object.
//  {a:b, key:[... --> {a:b, key:[...]
#[macro_export]
macro_rules! json_rep_close_array {
  ($object:ident, $key:ident) => {{
    concat!(
      "begin json_rep_close_array ",
      ", object: ", stringify!($object),
      ", key: ",    stringify!($key),
      ", child: ",  stringify!($object), "_map"  //  object##_map
    );
    json_encode_array_finish(&coap_json_encoder);

    //  d!(> TODO: json_rep_end_array(object##_map, key));
    //  json_rep_end_array!($object, $key, _map);

    d!(end json_rep_close_array);
  }};
}

#[macro_export]
macro_rules! json_rep_object_array_start_item {
  ($key:ident) => {{
    concat!(
      "begin json_rep_object_array_start_item ",
      ", key: ",    stringify!($key),
      ", child: ",  stringify!($key), "_array",  //  key##_array
    );
    { json_encode_object_start(&coap_json_encoder); }

    //  d!(> TODO: json_rep_start_object(key##_array, key));        
    //  json_rep_start_object!($key, $key, _array);

    d!(end json_rep_object_array_start_item);
  }};
}

#[macro_export]
macro_rules! json_rep_object_array_end_item {
  ($key:ident) => {{
    concat!(
      "begin json_rep_object_array_end_item ",
      ", key: ",    stringify!($key),
      ", child: ",  stringify!($key), "_array",  //  key##_array
    );
    { json_encode_object_finish(&coap_json_encoder); }   

    //  d!(> TODO: json_rep_end_object(key##_array, key));
    //  json_rep_end_object!($key, $key, _array);

    d!(end json_rep_object_array_end_item);
  }};
}

#[macro_export]
macro_rules! json_rep_set_int {
  ($object:ident, $key:expr, $value:expr) => {{
    concat!(
      "begin json_rep_set_int ",
      ", object: ", stringify!($object),
      ", key: ",    stringify!($key),
      ", value: ",  stringify!($value),
      ", child: ",  stringify!($object), "_map"  //  object##_map
    );
    unsafe {
      json_value_int(&coap_json_value, value);          
      json_encode_object_entry(&coap_json_encoder, #key, &coap_json_value);

      //  d!(> TODO: g_err |= cbor_encode_text_string(&object##_map, #key, strlen(#key)));
      //  cbor_encode_text_string(&mut concat_idents!($object,_map), $key.as_ptr(), $key.len());
      //  d!(> TODO: g_err |= cbor_encode_int(&object##_map, value));
      //  cbor_encode_int(&mut concat_idents!($object,_map), $value);
    }
    d!(end json_rep_set_int);
  }};
}

#[macro_export]
macro_rules! json_rep_set_text_string {
  ($object:ident, $key:expr, $value:expr) => {{
    concat!(
      "begin json_rep_set_text_string ",
      ", object: ", stringify!($object),
      ", key: ",    stringify!($key),
      ", value: ",  stringify!($value),
      ", child: ",  stringify!($object), "_map"  //  object##_map
    );
    unsafe {
      json_value_string(&coap_json_value, (char *) value); 
      json_encode_object_entry(&coap_json_encoder, #key, &coap_json_value);

      //  d!(> TODO: g_err |= cbor_encode_text_string(&object##_map, #key, strlen(#key)));
      //  cbor_encode_text_string(&mut concat_idents!($object, _map), $key.as_ptr(), $key.len());
      //  d!(> TODO: g_err |= cbor_encode_text_string(&object##_map, value, strlen(value)));
      //  cbor_encode_text_string(&mut concat_idents!($object, _map), $value.as_ptr(), $value.len());
    }
    d!(end json_rep_set_text_string);
  }};
}

///////////////////////////////////////////////////////////////////////////////
//  JSON Encoding macros ported from C to Rust:
//  https://github.com/apache/mynewt-core/blob/master/encoding/json/include/json/json.h

#[macro_export]
macro_rules! json_value_int {
  ($json_value:ident, $value:expr) => {{
    concat!(
      "begin json_value_int ",
      ", json_value: ", stringify!($json_value),
      ", value: ",  stringify!($value)
    );
    unsafe {
      $json_value->jv_type = JSON_VALUE_TYPE_INT64;
      $json_value->jv_val.u = (uint64_t) $value;
    }
    d!(end json_value_int);
  }};
}

#[macro_export]
macro_rules! json_value_string {
  ($json_value:ident, $value:expr) => {{
    concat!(
      "begin json_value_string ",
      ", json_value: ", stringify!($json_value),
      ", value: ",  stringify!($value)
    );
    unsafe {
      $json_value->jv_type = JSON_VALUE_TYPE_STRING;
      $json_value->jv_len = strlen($value);
      $json_value->jv_val.str = ($value);
    }
    d!(end json_value_string);
  }};
}

///////////////////////////////////////////////////////////////////////////////
//  CBOR macros ported from C to Rust:
//  https://github.com/apache/mynewt-core/blob/master/net/oic/include/oic/oc_rep.h

#[macro_export(local_inner_macros)]
macro_rules! oc_rep_start_root_object {
  () => {{
    d!(begin oc_rep_start_root_object);
    //  TODO
    //  d!(> TODO: g_err |= cbor_encoder_create_map(&g_encoder, &root_map, CborIndefiniteLength));
    unsafe { cbor_encoder_create_map(&mut g_encoder, &mut root_map, CborIndefiniteLength) };
    d!(end oc_rep_start_root_object);
  }};
}

#[macro_export(local_inner_macros)]
macro_rules! oc_rep_end_root_object {
  () => {{
    d!(begin oc_rep_end_root_object);
    //  d!(> TODO: g_err |= cbor_encoder_close_container(&g_encoder, &root_map));
    unsafe { cbor_encoder_close_container(&mut g_encoder, &mut root_map); }
    d!(end oc_rep_end_root_object);
  }};
}

#[macro_export]
macro_rules! oc_rep_start_object {
  ($parent:ident, $key:ident, $parent_suffix:ident) => {{
    concat!(
      "begin oc_rep_start_object ",
      ", parent: ", stringify!($parent), stringify!($parent_suffix),  //  parent##parent_suffix
      ", key: ",    stringify!($key),
      ", child: ",  stringify!($key), "_map"  //  key##_map
    );
    //  d!(> TODO: CborEncoder key##_map);
    //  concat_idents!($key, _map) = CborEncoder{};
    //  d!(> TODO: g_err |= cbor_encoder_create_map(&parent, &key##_map, CborIndefiniteLength));
    unsafe { cbor_encoder_create_map(
      &mut $parent, 
      &mut concat_idents!($key, _map), 
      CborIndefiniteLength) 
    };
    d!(end oc_rep_start_object);
  }};
}

#[macro_export]
macro_rules! oc_rep_end_object {
  ($parent:ident, $key:ident, $parent_suffix:ident) => {{
    concat!(
      "begin oc_rep_end_object ",
      ", parent: ", stringify!($parent), stringify!($parent_suffix),  //  parent##parent_suffix
      ", key: ",    stringify!($key),
      ", child: ",  stringify!($key), "_map"  //  key##_map
    );
    //  d!(> TODO: g_err |= cbor_encoder_close_container(&parent, &key##_map));
    unsafe { cbor_encoder_close_container(
      &mut concat_idents!($parent, $parent_suffix), 
      &mut concat_idents!($key, _map)) 
    };
    d!(end oc_rep_end_object);
  }};
}

#[macro_export]
macro_rules! oc_rep_start_array {
  ($parent:ident, $key:ident, $parent_suffix:ident) => {{
    concat!(
      "begin oc_rep_start_array ",
      ", parent: ", stringify!($parent), stringify!($parent_suffix),  //  parent##parent_suffix
      ", key: ",    stringify!($key),
      ", child: ",  stringify!($key), "_array"  //  key##_array
    );
    //  d!(> TODO: CborEncoder key##_array);
    //  concat_idents!($key, _array) = CborEncoder{};
    //  d!(> TODO: g_err |= cbor_encoder_create_array(&parent, &key##_array, CborIndefiniteLength));
    unsafe { cbor_encoder_create_array(
      &mut concat_idents!($parent, $parent_suffix), 
      &mut concat_idents!($key, _array), 
      CborIndefiniteLength) 
    };
    d!(end oc_rep_start_array);
  }};
}

#[macro_export]
macro_rules! oc_rep_end_array {
  ($parent:ident, $key:ident, $parent_suffix:ident) => {{
    concat!(
      "begin oc_rep_end_array ",
      ", parent: ", stringify!($parent), stringify!($parent_suffix),  //  parent##parent_suffix
      ", key: ",    stringify!($key),
      ", child: ",  stringify!($key), "_array"  //  key##_array
    );
    //  d!(> TODO: g_err |= cbor_encoder_close_container(&parent, &key##_array));
    unsafe { cbor_encoder_close_container(
      &mut $parent, 
      &mut concat_idents!($parent, $parent_suffix)) 
    };
    d!(end oc_rep_end_array);
  }};
}

#[macro_export]
macro_rules! oc_rep_set_array {
  ($object:ident, $key:ident) => {{
    concat!(
      "begin oc_rep_set_array ",
      ", object: ", stringify!($object),
      ", key: ",    stringify!($key),
      ", child: ",  stringify!($object), "_map"  //  object##_map
    );
    //  concat!("> TODO: g_err |= cbor_encode_text_string(&object##_map, #key, strlen(#key));");
    unsafe { cbor_encode_text_string(&mut concat_idents!($object, _map), $key.as_ptr(), $key.len()) };
    //  concat!("> TODO: oc_rep_start_array!(object##_map, key);");
    oc_rep_start_array!($object, $key, _map);
    d!(end oc_rep_set_array);
  }};
}

#[macro_export]
macro_rules! oc_rep_close_array {
  ($object:ident, $key:ident) => {{
    concat!(
      "begin oc_rep_close_array ",
      ", object: ", stringify!($object),
      ", key: ",    stringify!($key),
      ", child: ",  stringify!($object), "_map"  //  object##_map
    );
    //  d!(> TODO: oc_rep_end_array(object##_map, key));
    oc_rep_end_array!($object, $key, _map);
    d!(end oc_rep_close_array);
  }};
}

#[macro_export]
macro_rules! oc_rep_object_array_start_item {
  ($key:ident) => {{
    concat!(
      "begin oc_rep_object_array_start_item ",
      ", key: ",    stringify!($key),
      ", child: ",  stringify!($key), "_array",  //  key##_array
    );
    //  d!(> TODO: oc_rep_start_object(key##_array, key));        
    oc_rep_start_object!($key, $key, _array);
    d!(end oc_rep_object_array_start_item);
  }};
}

#[macro_export]
macro_rules! oc_rep_object_array_end_item {
  ($key:ident) => {{
    concat!(
      "begin oc_rep_object_array_end_item ",
      ", key: ",    stringify!($key),
      ", child: ",  stringify!($key), "_array",  //  key##_array
    );
    //  d!(> TODO: oc_rep_end_object(key##_array, key));
    oc_rep_end_object!($key, $key, _array);
    d!(end oc_rep_object_array_end_item);
  }};
}

#[macro_export]
macro_rules! oc_rep_set_int {
  ($object:ident, $key:expr, $value:expr) => {{
    concat!(
      "begin oc_rep_set_int ",
      ", object: ", stringify!($object),
      ", key: ",    stringify!($key),
      ", value: ",  stringify!($value),
      ", child: ",  stringify!($object), "_map"  //  object##_map
    );
    unsafe {
      //  d!(> TODO: g_err |= cbor_encode_text_string(&object##_map, #key, strlen(#key)));
      cbor_encode_text_string(&mut concat_idents!($object,_map), $key.as_ptr(), $key.len());
      //  d!(> TODO: g_err |= cbor_encode_int(&object##_map, value));
      cbor_encode_int(&mut concat_idents!($object,_map), $value);
    }
    d!(end oc_rep_set_int);
  }};
}

/*
///  Same as oc_rep_set_int but changed "#key" to "key" so that the key won't be stringified.
#[macro_export]
macro_rules! oc_rep_set_int_k {
  ($object:ident, $key:expr, $value:expr) => {{
    concat!(
      "begin oc_rep_set_int_k ",
      ", object: ", stringify!($object),
      ", key: ",    stringify!($key),
      ", value: ",  stringify!($value),
      ", child: ",  stringify!($object), "_map"  //  object##_map
    );
    //  d!(> TODO: g_err |= cbor_encode_text_string(&object##_map, key, strlen(key)));
    concat!(
      "> TODO: g_err |= cbor_encode_text_string(&",
      stringify!($object), "_map",  //  object##_map
      ", ",
      stringify!($key),  //  key
      ", strlen(",
      stringify!($key),  //  key
      "));"
    );

    //  d!(> TODO: g_err |= cbor_encode_int(&object##_map, value));
    concat!(
      "> TODO: g_err |= cbor_encode_int(&",
      stringify!($object), "_map",  //  object##_map
      ", ",
      stringify!($value),  //  value
      ");"
    );
    d!(end oc_rep_set_int_k);
  }};
}
*/

#[macro_export]
macro_rules! oc_rep_set_text_string {
  ($object:ident, $key:expr, $value:expr) => {{
    concat!(
      "begin oc_rep_set_text_string ",
      ", object: ", stringify!($object),
      ", key: ",    stringify!($key),
      ", value: ",  stringify!($value),
      ", child: ",  stringify!($object), "_map"  //  object##_map
    );
    unsafe {
      //  d!(> TODO: g_err |= cbor_encode_text_string(&object##_map, #key, strlen(#key)));
      cbor_encode_text_string(&mut concat_idents!($object, _map), $key.as_ptr(), $key.len());
      //  d!(> TODO: g_err |= cbor_encode_text_string(&object##_map, value, strlen(value)));
      cbor_encode_text_string(&mut concat_idents!($object, _map), $value.as_ptr(), $value.len());
    }
    d!(end oc_rep_set_text_string);
  }};
}

///////////////////////////////////////////////////////////////////////////////
//  Test Macros

#[macro_export]
macro_rules! test_literal {
  ($key:literal) => {{
    concat!($key, "_zzz");
  }};
}

#[macro_export]
macro_rules! test_ident {
  ($key:ident) => {{
    let $key = stringify!($key);
    //  concat_idents!($key, _map);
  }};
}

#[macro_export]
macro_rules! test_internal_rules2 {
  (@json $key:ident) => {
    let _ = concat!("json2: ", stringify!($key));
  };
  (@cbor $key:ident) => {
    let _ = concat!("cbor2: ", stringify!($key));
  };
  (@$encoding:ident $key:ident) => {
    let _ = concat!("other2: ", stringify!($encoding), " / ", stringify!($key));
  };
}

#[macro_export]
macro_rules! test_internal_rules {
  (@json $key:ident) => {
    let _ = concat!("json: ", stringify!($key));
    test_internal_rules2!(@json $key);
  };
  (@cbor $key:ident) => {
    let _ = concat!("cbor: ", stringify!($key));
    test_internal_rules2!(@cbor $key);
  };
  (@$encoding:ident $key:ident) => {
    let _ = concat!("other: ", stringify!($encoding), " / ", stringify!($key));
    test_internal_rules2!(@$encoding $key);
  };
}

///////////////////////////////////////////////////////////////////////////////
//  Utility Macros

///  Macro to dump all tokens received as a literal string, e.g.
///  `d!(a b c)` returns `"a b c"`
#[macro_export]
macro_rules! d {
  //  This rule matches zero or more tokens.
  ($($token:tt)*) => {
    //  For all matched tokens, convert into a string.
    stringify!($($token)*)
  };
}

///  Macro to display the token being parsed and the remaining tokens
#[macro_export]
macro_rules! nx {
  (($($current:tt)*), ($($next:tt)*), ($($rest:tt)*)) => {
    concat!(
      " >> ",
      stringify!($($current)*), 
      " >> ",
      stringify!($($next)*), 
      " >> ",
      stringify!($($rest)*)
    );
  };
}