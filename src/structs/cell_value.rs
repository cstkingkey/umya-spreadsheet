use super::RichText;
use super::SharedStringItem;
use helper::formula::*;
use md5::Digest;
use parking_lot::RwLock;

#[derive(Default, Debug)]
pub struct CellValue {
    pub(crate) value: RwLock<Option<Value>>,
    pub(crate) raw_value: Option<String>,
    pub(crate) rich_text: Option<RichText>,
    pub(crate) formula: Option<String>,
    pub(crate) formula_attributes: Vec<(String, String)>,
}

impl PartialEq for CellValue {
    fn eq(&self, other: &Self) -> bool {
        *self.value.read() == *other.value.read() && self.raw_value == other.raw_value && self.rich_text == other.rich_text && self.formula == other.formula && self.formula_attributes == other.formula_attributes
    }
}

impl PartialOrd for CellValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (*self.value.read()).partial_cmp(&other.value.read()) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.raw_value.partial_cmp(&other.raw_value) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.rich_text.partial_cmp(&other.rich_text) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.formula.partial_cmp(&other.formula) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.formula_attributes.partial_cmp(&other.formula_attributes)
    }
}

impl Clone for CellValue {
    fn clone(&self) -> Self {
        let v= self.value.read().clone();
        let v = RwLock::new(v);
        Self { value: v, raw_value: self.raw_value.clone(), rich_text: self.rich_text.clone(), formula: self.formula.clone(), formula_attributes: self.formula_attributes.clone() }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Value {
    String(String),
    //Formula(String),
    Numeric(f64),
    Bool(bool),
    Null,
    Inline,
    Error,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Numeric(ft) => write!(f, "{}", ft),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Null => write!(f, ""),
            Value::Inline => write!(f, ""),
            Value::Error => write!(f, ""),
        }
    }
}

impl CellValue {
    // Data types
    pub const TYPE_STRING2: &'static str = "str";
    pub const TYPE_STRING: &'static str = "s";
    pub const TYPE_FORMULA: &'static str = "f";
    pub const TYPE_NUMERIC: &'static str = "n";
    pub const TYPE_BOOL: &'static str = "b";
    pub const TYPE_NULL: &'static str = "null";
    pub const TYPE_INLINE: &'static str = "inlineStr";
    pub const TYPE_ERROR: &'static str = "e";

    pub fn set_formula_attributes(&mut self, formula_attributes: Vec<(String, String)>) {
        self.formula_attributes = formula_attributes;
    }
    pub fn get_formula_attributes(&self) -> Vec<(&str, &str)> {
        self.formula_attributes
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect()
    }

    pub fn get_typed_value(&self) -> Option<Value> {
        if self.value.read().is_none() {
            if let Some(rv) = self.raw_value.as_ref() {
                let tv = Self::guess_typed_data(&rv);
                *self.value.write() = Some(tv);
            }
        }

        (*self.value.read()).clone()
    }

    pub fn get_value(&self) -> String {
        match self.get_typed_value() {
            Some(v) => {
                return v.to_string();
            }
            None => {}
        }
        match &self.rich_text {
            Some(v) => {
                return v.get_text().to_string();
            }
            None => {}
        }
        "".to_string()
    }

    //pub(crate) fn get_value_crate(&self) -> &Option<String> {
    //    &self.value
    //}

    pub fn get_rich_text(&self) -> &Option<RichText> {
        &self.rich_text
    }

    pub fn set_value<S: AsRef<str>>(&mut self, value: S) -> &mut Self {
        let value = Self::guess_typed_data(value.as_ref());
        *self.value.write() = Some(value);
        self.rich_text = None;
        self.formula = None;
        self
    }

    pub fn set_value_raw<S: Into<String>>(&mut self, value: S) -> &mut Self {
        self.raw_value = Some(value.into());
        self
    }

    pub fn set_value_from_string<S: Into<String>>(&mut self, value: S) -> &mut Self {
        *self.value.write() = Some(Value::String(value.into()));
        self.rich_text = None;
        self.formula = None;
        self
    }

    pub fn set_value_from_bool(&mut self, value: bool) -> &mut Self {
        *self.value.write() = Some(Value::Bool(value));
        self.rich_text = None;
        self.formula = None;
        self
    }

    pub fn set_value_from_bool_ref(&mut self, value: &bool) -> &mut Self {
        self.set_value_from_bool(*value)
    }

    pub fn set_value_from_numberic<V: Into<f64>>(&mut self, value: V) -> &mut Self {
        *self.value.write() = Some(Value::Numeric(value.into()));
        self.rich_text = None;
        self.formula = None;
        self
    }

    pub fn set_rich_text(&mut self, value: RichText) -> &mut Self {
        *self.value.write() = None;
        self.rich_text = Some(value);
        self.formula = None;
        self
    }

    pub fn set_rich_text_ref(&mut self, value: &RichText) -> &mut Self {
        self.set_rich_text(value.clone())
    }

    pub fn set_formula<S: Into<String>>(&mut self, value: S) -> &mut Self {
        *self.value.write() = None;
        self.rich_text = None;
        self.formula = Some(value.into());
        self
    }

    pub(crate) fn set_shared_string_item(&mut self, value: SharedStringItem) -> &mut Self {
        match value.get_text() {
            Some(v) => {
                *self.value.write() = Some(Value::String(v.get_value().to_string()));
            }
            None => {}
        }
        self.rich_text = value.get_rich_text().clone();
        self.formula = None;
        self
    }

    pub fn get_data_type(&self) -> &str {
        let value = self.get_typed_value();
        
        if let Some(v) = value {
            let r = match v {
                Value::String(_) => Self::TYPE_STRING,
                Value::Numeric(_) => Self::TYPE_NUMERIC,
                Value::Bool(_) => Self::TYPE_BOOL,
                Value::Null => Self::TYPE_NULL,
                Value::Inline => Self::TYPE_INLINE,
                Value::Error => Self::TYPE_ERROR,
            };
            return r;
        } else {
            if self.formula.is_some() {
                return  Self::TYPE_STRING
            }

            if self.rich_text.is_some() {
                return Self::TYPE_STRING
            }
        }
        
        Self::TYPE_STRING
    }

    pub fn set_data_type<S: Into<String>>(&mut self, value: S) -> &mut Self {
        todo!();
        self
    }

    pub(crate) fn check_data_type<S: Into<String>>(
        value: S,
        data_type: S,
    ) -> Result<(), &'static str> {
        match data_type.into().as_str() {
            Self::TYPE_STRING2 => Ok(()),
            Self::TYPE_STRING => Ok(()),
            Self::TYPE_FORMULA => Ok(()),
            Self::TYPE_NUMERIC => match &value.into().parse::<f64>() {
                Ok(_) => Ok(()),
                Err(_) => Err("Invalid numeric value for datatype Numeric"),
            },
            Self::TYPE_BOOL => {
                let check_value = &value.into().to_uppercase();
                if check_value == "TRUE" || check_value == "FALSE" {
                    Ok(())
                } else {
                    Err("Invalid value for datatype Bool")
                }
            }
            Self::TYPE_NULL => Ok(()),
            _ => Err("Invalid datatype"),
        }
    }

    pub fn is_formula(&self) -> bool {
        self.formula.is_some()
    }

    pub fn get_formula(&self) -> &str {
        match &self.formula {
            Some(v) => {
                return v;
            }
            None => {}
        }
        ""
    }

    pub(crate) fn data_type_for_value(value: &str) -> &str {
        let check_value = value.to_uppercase();

        // Match the value against a few data types
        if check_value == "NULL" {
            return Self::TYPE_NULL;
        }
        match check_value.parse::<f64>() {
            Ok(_) => return Self::TYPE_NUMERIC,
            Err(_) => {}
        }
        if check_value == "TRUE" || check_value == "FALSE" {
            return Self::TYPE_BOOL;
        }
        Self::TYPE_STRING
    }

    pub(crate) fn guess_typed_data(value: &str) -> Value {
        let uppercase_value = value.to_uppercase();

        // Match the value against a few data types
        if uppercase_value == "NULL" {
            return Value::Null;
        }

        if let Ok(f) = value.parse::<f64>() {
            return Value::Numeric(f);
        }

        if uppercase_value == "TRUE" {
            return  Value::Bool(true);
        }

        if uppercase_value == "FALSE" {
            return  Value::Bool(false);
        }

        Value::String(value.into())
    }

    // pub(crate) fn _get_hash_code_by_value(&self) -> String {
    //     format!(
    //         "{:x}",
    //         md5::Md5::digest(format!(
    //             "{}{}",
    //             match &self.value {
    //                 Some(v) => {
    //                     v
    //                 }
    //                 None => {
    //                     "None"
    //                 }
    //             },
    //             match &self.rich_text {
    //                 Some(v) => {
    //                     v.get_hash_code()
    //                 }
    //                 None => {
    //                     "None".into()
    //                 }
    //             },
    //         ))
    //     )
    // }

    pub(crate) fn is_empty(&self) -> bool {
        match self.get_typed_value() {
            Some(_) => return false,
            None => {}
        }
        match &self.rich_text {
            Some(_) => return false,
            None => {}
        }
        match &self.formula {
            Some(_) => return false,
            None => {}
        }
        true
    }

    pub(crate) fn adjustment_insert_formula_coordinate(
        &mut self,
        self_sheet_name: &str,
        sheet_name: &str,
        root_col_num: &u32,
        offset_col_num: &u32,
        root_row_num: &u32,
        offset_row_num: &u32,
    ) {
        match &self.formula {
            Some(v) => {
                let formula = adjustment_insert_formula_coordinate(
                    v,
                    root_col_num,
                    offset_col_num,
                    root_row_num,
                    offset_row_num,
                    sheet_name,
                    self_sheet_name,
                );
                self.formula = Some(formula);
            }
            None => {}
        }
    }

    pub(crate) fn adjustment_remove_formula_coordinate(
        &mut self,
        self_sheet_name: &str,
        sheet_name: &str,
        root_col_num: &u32,
        offset_col_num: &u32,
        root_row_num: &u32,
        offset_row_num: &u32,
    ) {
        match &self.formula {
            Some(v) => {
                let formula = adjustment_remove_formula_coordinate(
                    v,
                    root_col_num,
                    offset_col_num,
                    root_row_num,
                    offset_row_num,
                    sheet_name,
                    self_sheet_name,
                );
                self.formula = Some(formula);
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_value() {
        let mut obj = CellValue::default();

        obj.set_value_from_string(String::from("TEST"));
        assert_eq!(obj.get_value(), "TEST");

        obj.set_value_from_string("TEST");
        assert_eq!(obj.get_value(), "TEST");

        obj.set_value_from_bool(true);
        assert_eq!(obj.get_value(), "true");

        obj.set_value_from_bool_ref(&true);
        assert_eq!(obj.get_value(), "true");

        obj.set_value_from_numberic(1);
        assert_eq!(obj.get_value(), "1");

        obj.set_value_from_numberic(1.09);
        assert_eq!(obj.get_value(), "1.09");
    }
}
