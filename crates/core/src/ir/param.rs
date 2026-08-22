use crate::ColumnTypeReference;

use super::{CoreType, CoreTypeRef};

/// Database-independent runtime encoding for one bound Param value.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ParamEncoding {
    /// Pass the target-language input value through unchanged.
    #[default]
    Identity,
    /// Encode a logical boolean as database integer `0` or `1`.
    BooleanAsInteger,
}

/// One generated parameter binding in source occurrence order.
#[derive(Clone, Debug)]
pub struct ParamBinding {
    input_name: String,
    type_ref: CoreTypeRef,
    nullable: bool,
    encoding: ParamEncoding,
    schema_column_reference: Option<ColumnTypeReference>,
}

impl PartialEq for ParamBinding {
    fn eq(&self, other: &Self) -> bool {
        // Schema column references are provenance metadata and do not define Param identity.
        self.input_name == other.input_name
            && self.type_ref == other.type_ref
            && self.nullable == other.nullable
            && self.encoding == other.encoding
    }
}

impl Eq for ParamBinding {}

impl ParamBinding {
    /// Build a query parameter binding.
    #[must_use]
    pub const fn new(input_name: String, ty: CoreType, nullable: bool) -> Self {
        Self::new_type_ref(input_name, CoreTypeRef::Scalar(ty), nullable)
    }

    /// Build a query parameter binding from a richer Core type reference.
    #[must_use]
    pub const fn new_type_ref(input_name: String, type_ref: CoreTypeRef, nullable: bool) -> Self {
        Self {
            input_name,
            type_ref,
            nullable,
            encoding: ParamEncoding::Identity,
            schema_column_reference: None,
        }
    }

    /// Attach the database value encoding for this Param occurrence.
    #[must_use]
    pub const fn with_encoding(mut self, encoding: ParamEncoding) -> Self {
        self.encoding = encoding;
        self
    }

    /// Attach the schema column reference that supplied this Param type, when available.
    #[must_use]
    pub fn with_schema_column_reference(mut self, reference: ColumnTypeReference) -> Self {
        self.schema_column_reference = Some(reference);
        self
    }

    /// Input field name used for this parameter occurrence.
    #[must_use]
    pub fn input_name(&self) -> &str {
        &self.input_name
    }

    /// Language-neutral parameter type.
    #[must_use]
    pub const fn ty(&self) -> CoreType {
        self.type_ref.core_type()
    }

    /// Language-neutral parameter type reference.
    #[must_use]
    pub const fn type_ref(&self) -> &CoreTypeRef {
        &self.type_ref
    }

    /// Whether this parameter occurrence accepts null.
    #[must_use]
    pub const fn is_nullable(&self) -> bool {
        self.nullable
    }

    /// Database-independent runtime encoding for the bound value.
    #[must_use]
    pub const fn encoding(&self) -> ParamEncoding {
        self.encoding
    }

    /// Schema column reference that supplied this Param type, when available.
    #[must_use]
    pub const fn schema_column_reference(&self) -> Option<&ColumnTypeReference> {
        self.schema_column_reference.as_ref()
    }
}
