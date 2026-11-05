//! Rejects TOML arrays where a Config File section expects a table.
//!
//! A derived [`Deserialize`] struct accepts a sequence as well as a map,
//! because Serde reserves the sequence form for formats without field names.
//! TOML arrays are self-describing, but `toml` forwards them to the struct's
//! sequence visitor, so `templates = ["x"]` would otherwise fill the struct by
//! position. The `deserialize_with` functions here pass only maps to the
//! struct's visitor. Serde then reports `invalid type: sequence, expected
//! struct <Name>` and `toml` keeps the span of the offending value.

use std::fmt;

use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, Visitor},
};

/// Forwards only maps to the wrapped struct visitor.
///
/// Every other `visit_*` method keeps the [`Visitor`] default, which fails with
/// an `invalid type` error that names the wrapped visitor's
/// [`expecting`](Visitor::expecting) text.
struct MapOnly<V>(V);

impl<'de, V: Visitor<'de>> Visitor<'de> for MapOnly<V> {
    type Value = V::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.expecting(formatter)
    }

    fn visit_map<A: MapAccess<'de>>(
        self,
        map: A,
    ) -> Result<Self::Value, A::Error> {
        self.0.visit_map(map)
    }
}

/// Wraps a [`Deserializer`] so that `deserialize_struct` accepts only a map.
///
/// Every other entry point forwards to `deserialize_any`. The wrapped type must
/// therefore be a struct, which is the only shape this module handles.
struct TableOnly<D>(D);

impl<'de, D: Deserializer<'de>> Deserializer<'de> for TableOnly<D> {
    type Error = D::Error;

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map enum identifier ignored_any
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.deserialize_struct(name, fields, MapOnly(visitor))
    }

    fn deserialize_any<V: Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.deserialize_any(visitor)
    }
}

/// Deserializes a struct from a TOML table only.
///
/// `#[repr(transparent)]` keeps the layout equal to `T`, so [`table_list`] can
/// reuse the `Vec` allocation when it unwraps the entries.
#[repr(transparent)]
struct Table<T>(T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Table<T> {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        T::deserialize(TableOnly(deserializer)).map(Self)
    }
}

/// Deserializes a required table field into `T` and rejects an array.
///
/// Use it as `deserialize_with` on a field that holds a [`Deserialize`] struct.
///
/// # Errors
///
/// - [`Deserializer::Error`] if the value is not a table.
/// - [`Deserializer::Error`] if `T` rejects the keys of the table.
pub(super) fn table<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Table::<T>::deserialize(deserializer).map(|table| table.0)
}

/// Deserializes an optional table field into `Option<T>` and rejects an array.
///
/// Use it as `deserialize_with` together with `default` on a field that holds
/// an optional [`Deserialize`] struct.
///
/// # Errors
///
/// - [`Deserializer::Error`] if a present value is not a table.
/// - [`Deserializer::Error`] if `T` rejects the keys of the table.
pub(super) fn opt_table<'de, D, T>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<Table<T>>::deserialize(deserializer)
        .map(|table| table.map(|table| table.0))
}

/// Deserializes an array of tables into `Vec<T>` and rejects an array entry
/// that is not a table.
///
/// Use it as `deserialize_with` on a field that holds a list of [`Deserialize`]
/// structs.
///
/// # Errors
///
/// - [`Deserializer::Error`] if the value is not an array.
/// - [`Deserializer::Error`] if an entry is not a table that `T` accepts.
pub(super) fn table_list<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Vec::<Table<T>>::deserialize(deserializer)
        .map(|tables| tables.into_iter().map(|table| table.0).collect())
}
