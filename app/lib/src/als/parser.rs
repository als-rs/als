//! ALS format parser.
//!
//! This module provides the parser for converting ALS format text into
//! `AlsDocument` structures and expanding them to tabular data.

use crate::config::ParserConfig;
use crate::error::{AlsError, Result};

use super::document::{AlsDocument, ColumnStream, FormatIndicator};
use super::operator::AlsOperator;
use super::tokenizer::{Token, Tokenizer, VersionType};

/// ALS format parser.
///
/// Parses ALS format text into `AlsDocument` structures and can expand
/// them to tabular data (CSV, JSON).
pub struct AlsParser {
    config: ParserConfig,
}

impl AlsParser {
    /// Current maximum supported ALS version.
    pub const MAX_SUPPORTED_VERSION: u8 = 1;

    /// Create a new parser with default configuration.
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
        }
    }

    /// Create a new parser with the given configuration.
    pub fn with_config(config: ParserConfig) -> Self {
        Self { config }
    }

    /// Parse ALS format text into an `AlsDocument`.
    pub fn parse(&self, input: &str) -> Result<AlsDocument> {
        let mut tokenizer = Tokenizer::new(input);
        self.parse_document(&mut tokenizer)
    }

    /// Parse a complete ALS document from the tokenizer.
    fn parse_document(&self, tokenizer: &mut Tokenizer) -> Result<AlsDocument> {
        let mut doc = AlsDocument::new();

        // Parse optional version
        self.skip_whitespace_tokens(tokenizer)?;
        if let Token::Version(version_type) = tokenizer.peek_token()? {
            tokenizer.next_token()?; // consume version
            match version_type {
                VersionType::Als(v) => {
                    if v > Self::MAX_SUPPORTED_VERSION {
                        return Err(AlsError::VersionMismatch {
                            expected: Self::MAX_SUPPORTED_VERSION,
                            found: v,
                        });
                    }
                    doc.version = v;
                    doc.format_indicator = FormatIndicator::Als;
                }
                VersionType::Ctx => {
                    doc.format_indicator = FormatIndicator::Ctx;
                }
            }
            self.skip_whitespace_tokens(tokenizer)?;
        }

        // Parse optional dictionaries
        while let Token::DictionaryHeader { name, values } = tokenizer.peek_token()? {
            tokenizer.next_token()?; // consume dictionary header
            doc.dictionaries.insert(name, values);
            self.skip_whitespace_tokens(tokenizer)?;
        }

        // Parse schema
        while let Token::SchemaColumn(name) = tokenizer.peek_token()? {
            tokenizer.next_token()?; // consume schema column
            doc.schema.push(name);
        }
        self.skip_whitespace_tokens(tokenizer)?;

        // Parse streams
        if !doc.schema.is_empty() {
            let streams = self.parse_streams(tokenizer, doc.schema.len())?;
            doc.streams = streams;
        }

        Ok(doc)
    }

    /// Skip newline tokens.
    fn skip_whitespace_tokens(&self, tokenizer: &mut Tokenizer) -> Result<()> {
        loop {
            match tokenizer.peek_token()? {
                Token::Newline => {
                    tokenizer.next_token()?;
                }
                _ => break,
            }
        }
        Ok(())
    }

    /// Parse column streams separated by |.
    fn parse_streams(&self, tokenizer: &mut Tokenizer, expected_columns: usize) -> Result<Vec<ColumnStream>> {
        let mut streams = Vec::with_capacity(expected_columns);
        let mut current_stream = ColumnStream::new();

        loop {
            let token = tokenizer.next_token()?;
            
            match token {
                Token::Eof => {
                    // End of input - save current stream if not empty
                    if !current_stream.is_empty() || streams.is_empty() {
                        streams.push(current_stream);
                    }
                    break;
                }
                Token::ColumnSeparator => {
                    // Save current stream and start new one
                    streams.push(current_stream);
                    current_stream = ColumnStream::new();
                }
                Token::Newline => {
                    // Skip newlines in stream section
                    continue;
                }
                _ => {
                    // Parse an element and add to current stream
                    let operator = self.parse_element(tokenizer, token)?;
                    current_stream.push(operator);
                }
            }
        }

        // Validate column count
        if streams.len() != expected_columns && expected_columns > 0 {
            return Err(AlsError::ColumnMismatch {
                schema: expected_columns,
                data: streams.len(),
            });
        }

        Ok(streams)
    }

    /// Parse a single element (operator or value).
    fn parse_element(&self, tokenizer: &mut Tokenizer, first_token: Token) -> Result<AlsOperator> {
        match first_token {
            Token::Integer(n) => self.parse_integer_element(tokenizer, n),
            Token::Float(f) => self.parse_float_element(tokenizer, f),
            Token::RawValue(s) => self.parse_raw_element(tokenizer, s),
            Token::DictRef(idx) => Ok(AlsOperator::dict_ref(idx)),
            Token::OpenParen => self.parse_grouped_element(tokenizer),
            _ => Err(AlsError::AlsSyntaxError {
                position: tokenizer.position(),
                message: format!("Unexpected token: {:?}", first_token),
            }),
        }
    }

    /// Parse an element starting with an integer (could be range, multiply, or raw).
    fn parse_integer_element(&self, tokenizer: &mut Tokenizer, start: i64) -> Result<AlsOperator> {
        match tokenizer.peek_token()? {
            Token::RangeOp => {
                tokenizer.next_token()?; // consume >
                self.parse_range(tokenizer, start)
            }
            Token::MultiplyOp => {
                tokenizer.next_token()?; // consume *
                let count = self.expect_integer(tokenizer)?;
                Ok(AlsOperator::multiply(AlsOperator::raw(start.to_string()), count as usize))
            }
            Token::ToggleOp => {
                tokenizer.next_token()?; // consume ~
                self.parse_toggle(tokenizer, start.to_string())
            }
            _ => Ok(AlsOperator::raw(start.to_string())),
        }
    }

    /// Parse an element starting with a float.
    fn parse_float_element(&self, tokenizer: &mut Tokenizer, value: f64) -> Result<AlsOperator> {
        match tokenizer.peek_token()? {
            Token::MultiplyOp => {
                tokenizer.next_token()?; // consume *
                let count = self.expect_integer(tokenizer)?;
                Ok(AlsOperator::multiply(AlsOperator::raw(value.to_string()), count as usize))
            }
            Token::ToggleOp => {
                tokenizer.next_token()?; // consume ~
                self.parse_toggle(tokenizer, value.to_string())
            }
            _ => Ok(AlsOperator::raw(value.to_string())),
        }
    }

    /// Parse an element starting with a raw value.
    fn parse_raw_element(&self, tokenizer: &mut Tokenizer, value: String) -> Result<AlsOperator> {
        match tokenizer.peek_token()? {
            Token::MultiplyOp => {
                tokenizer.next_token()?; // consume *
                let count = self.expect_integer(tokenizer)?;
                Ok(AlsOperator::multiply(AlsOperator::raw(value), count as usize))
            }
            Token::ToggleOp => {
                tokenizer.next_token()?; // consume ~
                self.parse_toggle(tokenizer, value)
            }
            _ => Ok(AlsOperator::raw(value)),
        }
    }

    /// Parse a range expression: start>end or start>end:step
    fn parse_range(&self, tokenizer: &mut Tokenizer, start: i64) -> Result<AlsOperator> {
        let end = self.expect_integer(tokenizer)?;
        
        let step = if let Token::StepSeparator = tokenizer.peek_token()? {
            tokenizer.next_token()?; // consume :
            self.expect_integer(tokenizer)?
        } else {
            if end >= start { 1 } else { -1 }
        };

        // Check for multiply after range
        let range_op = AlsOperator::range_safe_with_limit(
            start,
            end,
            step,
            self.config.max_range_expansion,
        )?;

        if let Token::MultiplyOp = tokenizer.peek_token()? {
            tokenizer.next_token()?; // consume *
            let count = self.expect_integer(tokenizer)?;
            Ok(AlsOperator::multiply(range_op, count as usize))
        } else {
            Ok(range_op)
        }
    }

    /// Parse a toggle expression: val1~val2[~val3...]*count
    fn parse_toggle(&self, tokenizer: &mut Tokenizer, first_value: String) -> Result<AlsOperator> {
        let mut values = vec![first_value];
        
        // Parse second value
        let second = self.expect_value(tokenizer)?;
        values.push(second);

        // Parse additional toggle values
        while let Token::ToggleOp = tokenizer.peek_token()? {
            tokenizer.next_token()?; // consume ~
            let next_value = self.expect_value(tokenizer)?;
            values.push(next_value);
        }

        // Parse optional count
        let count = if let Token::MultiplyOp = tokenizer.peek_token()? {
            tokenizer.next_token()?; // consume *
            self.expect_integer(tokenizer)? as usize
        } else {
            values.len() // Default to one cycle
        };

        Ok(AlsOperator::toggle_multi(values, count))
    }

    /// Parse a grouped element: (element)
    fn parse_grouped_element(&self, tokenizer: &mut Tokenizer) -> Result<AlsOperator> {
        let inner_token = tokenizer.next_token()?;
        let inner = self.parse_element(tokenizer, inner_token)?;
        
        // Expect closing paren
        match tokenizer.next_token()? {
            Token::CloseParen => {}
            other => {
                return Err(AlsError::AlsSyntaxError {
                    position: tokenizer.position(),
                    message: format!("Expected ')' but found {:?}", other),
                });
            }
        }

        // Check for multiply after group
        if let Token::MultiplyOp = tokenizer.peek_token()? {
            tokenizer.next_token()?; // consume *
            let count = self.expect_integer(tokenizer)?;
            Ok(AlsOperator::multiply(inner, count as usize))
        } else {
            Ok(inner)
        }
    }

    /// Expect and consume an integer token.
    fn expect_integer(&self, tokenizer: &mut Tokenizer) -> Result<i64> {
        match tokenizer.next_token()? {
            Token::Integer(n) => Ok(n),
            other => Err(AlsError::AlsSyntaxError {
                position: tokenizer.position(),
                message: format!("Expected integer but found {:?}", other),
            }),
        }
    }

    /// Expect and consume a value token (integer, float, or raw).
    fn expect_value(&self, tokenizer: &mut Tokenizer) -> Result<String> {
        match tokenizer.next_token()? {
            Token::Integer(n) => Ok(n.to_string()),
            Token::Float(f) => Ok(f.to_string()),
            Token::RawValue(s) => Ok(s),
            other => Err(AlsError::AlsSyntaxError {
                position: tokenizer.position(),
                message: format!("Expected value but found {:?}", other),
            }),
        }
    }

    /// Expand an ALS document to a vector of rows.
    ///
    /// Each row is a vector of string values.
    pub fn expand(&self, doc: &AlsDocument) -> Result<Vec<Vec<String>>> {
        if doc.streams.is_empty() {
            return Ok(Vec::new());
        }

        // Get the default dictionary for resolving references
        let default_dict = doc.default_dictionary();

        // Expand all columns
        let mut expanded_columns: Vec<Vec<String>> = Vec::with_capacity(doc.streams.len());
        for stream in &doc.streams {
            let column_values = stream.expand(default_dict.map(|v| v.as_slice()))?;
            expanded_columns.push(column_values);
        }

        // Validate all columns have the same length
        if let Some(first) = expanded_columns.first() {
            let expected_len = first.len();
            for col in expanded_columns.iter() {
                if col.len() != expected_len {
                    return Err(AlsError::ColumnMismatch {
                        schema: expected_len,
                        data: col.len(),
                    });
                }
            }
        }

        // Transpose columns to rows
        let row_count = expanded_columns.first().map(|c| c.len()).unwrap_or(0);
        let mut rows = Vec::with_capacity(row_count);
        
        for row_idx in 0..row_count {
            let row: Vec<String> = expanded_columns
                .iter()
                .map(|col| col[row_idx].clone())
                .collect();
            rows.push(row);
        }

        Ok(rows)
    }

    /// Parse ALS and expand directly to rows.
    pub fn parse_and_expand(&self, input: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        let doc = self.parse(input)?;
        let rows = self.expand(&doc)?;
        Ok((doc.schema.clone(), rows))
    }
}

impl Default for AlsParser {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_document() {
        let parser = AlsParser::new();
        let doc = parser.parse("").unwrap();
        assert!(doc.schema.is_empty());
        assert!(doc.streams.is_empty());
    }

    #[test]
    fn test_parse_version_als() {
        let parser = AlsParser::new();
        let doc = parser.parse("!v1\n#col\n1").unwrap();
        assert_eq!(doc.version, 1);
        assert_eq!(doc.format_indicator, FormatIndicator::Als);
    }

    #[test]
    fn test_parse_version_ctx() {
        let parser = AlsParser::new();
        let doc = parser.parse("!ctx\n#col\n1").unwrap();
        assert_eq!(doc.format_indicator, FormatIndicator::Ctx);
    }

    #[test]
    fn test_parse_unsupported_version() {
        let parser = AlsParser::new();
        let result = parser.parse("!v99\n#col\n1");
        assert!(matches!(result, Err(AlsError::VersionMismatch { .. })));
    }

    #[test]
    fn test_parse_dictionary() {
        let parser = AlsParser::new();
        let doc = parser.parse("$default:apple|banana|cherry\n#col\n_0").unwrap();
        assert!(doc.dictionaries.contains_key("default"));
        assert_eq!(doc.dictionaries["default"], vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn test_parse_schema() {
        let parser = AlsParser::new();
        let doc = parser.parse("#name #age #city\n1|2|3").unwrap();
        assert_eq!(doc.schema, vec!["name", "age", "city"]);
    }

    #[test]
    fn test_parse_raw_values() {
        let parser = AlsParser::new();
        let doc = parser.parse("#col\nhello world foo").unwrap();
        assert_eq!(doc.streams.len(), 1);
        assert_eq!(doc.streams[0].expanded_count(), 3);
    }

    #[test]
    fn test_parse_range() {
        let parser = AlsParser::new();
        let doc = parser.parse("#col\n1>5").unwrap();
        let expanded = doc.streams[0].expand(None).unwrap();
        assert_eq!(expanded, vec!["1", "2", "3", "4", "5"]);
    }

    #[test]
    fn test_parse_range_with_step() {
        let parser = AlsParser::new();
        let doc = parser.parse("#col\n10>50:10").unwrap();
        let expanded = doc.streams[0].expand(None).unwrap();
        assert_eq!(expanded, vec!["10", "20", "30", "40", "50"]);
    }

    #[test]
    fn test_parse_descending_range() {
        let parser = AlsParser::new();
        let doc = parser.parse("#col\n5>1:-1").unwrap();
        let expanded = doc.streams[0].expand(None).unwrap();
        assert_eq!(expanded, vec!["5", "4", "3", "2", "1"]);
    }

    #[test]
    fn test_parse_multiply() {
        let parser = AlsParser::new();
        let doc = parser.parse("#col\nhello*3").unwrap();
        let expanded = doc.streams[0].expand(None).unwrap();
        assert_eq!(expanded, vec!["hello", "hello", "hello"]);
    }

    #[test]
    fn test_parse_toggle() {
        let parser = AlsParser::new();
        let doc = parser.parse("#col\nT~F*4").unwrap();
        let expanded = doc.streams[0].expand(None).unwrap();
        assert_eq!(expanded, vec!["T", "F", "T", "F"]);
    }

    #[test]
    fn test_parse_dict_ref() {
        let parser = AlsParser::new();
        let doc = parser.parse("$default:red|green|blue\n#col\n_0 _1 _2").unwrap();
        let dict = doc.default_dictionary().unwrap();
        let expanded = doc.streams[0].expand(Some(dict)).unwrap();
        assert_eq!(expanded, vec!["red", "green", "blue"]);
    }

    #[test]
    fn test_parse_multiple_columns() {
        let parser = AlsParser::new();
        let doc = parser.parse("#id #name\n1>3|alice bob charlie").unwrap();
        assert_eq!(doc.streams.len(), 2);
        
        let col1 = doc.streams[0].expand(None).unwrap();
        let col2 = doc.streams[1].expand(None).unwrap();
        
        assert_eq!(col1, vec!["1", "2", "3"]);
        assert_eq!(col2, vec!["alice", "bob", "charlie"]);
    }

    #[test]
    fn test_parse_grouped_multiply() {
        let parser = AlsParser::new();
        let doc = parser.parse("#col\n(1>3)*2").unwrap();
        let expanded = doc.streams[0].expand(None).unwrap();
        assert_eq!(expanded, vec!["1", "2", "3", "1", "2", "3"]);
    }

    #[test]
    fn test_parse_range_multiply() {
        let parser = AlsParser::new();
        let doc = parser.parse("#col\n1>3*2").unwrap();
        let expanded = doc.streams[0].expand(None).unwrap();
        assert_eq!(expanded, vec!["1", "2", "3", "1", "2", "3"]);
    }

    #[test]
    fn test_expand_to_rows() {
        let parser = AlsParser::new();
        let doc = parser.parse("#id #name\n1>3|alice bob charlie").unwrap();
        let rows = parser.expand(&doc).unwrap();
        
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec!["1", "alice"]);
        assert_eq!(rows[1], vec!["2", "bob"]);
        assert_eq!(rows[2], vec!["3", "charlie"]);
    }

    #[test]
    fn test_parse_and_expand() {
        let parser = AlsParser::new();
        let (schema, rows) = parser.parse_and_expand("#id #name\n1>2|alice bob").unwrap();
        
        assert_eq!(schema, vec!["id", "name"]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["1", "alice"]);
        assert_eq!(rows[1], vec!["2", "bob"]);
    }

    #[test]
    fn test_column_mismatch_error() {
        let parser = AlsParser::new();
        let result = parser.parse("#col1 #col2 #col3\n1|2");
        assert!(matches!(result, Err(AlsError::ColumnMismatch { .. })));
    }

    #[test]
    fn test_parse_complex_document() {
        let input = r#"!v1
$default:active|inactive|pending
#id #name #status
1>5|alice*2 bob*2 charlie|_0 _1 _0 _1 _2"#;
        
        let parser = AlsParser::new();
        let doc = parser.parse(input).unwrap();
        
        assert_eq!(doc.version, 1);
        assert_eq!(doc.schema, vec!["id", "name", "status"]);
        assert_eq!(doc.streams.len(), 3);
        
        let rows = parser.expand(&doc).unwrap();
        
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0], vec!["1", "alice", "active"]);
        assert_eq!(rows[1], vec!["2", "alice", "inactive"]);
        assert_eq!(rows[2], vec!["3", "bob", "active"]);
        assert_eq!(rows[3], vec!["4", "bob", "inactive"]);
        assert_eq!(rows[4], vec!["5", "charlie", "pending"]);
    }

    #[test]
    fn test_parser_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AlsParser>();
    }

    #[test]
    fn test_version_detection_v1() {
        let parser = AlsParser::new();
        let doc = parser.parse("!v1\n#col\n1").unwrap();
        assert_eq!(doc.version, 1);
        assert!(doc.is_als());
    }

    #[test]
    fn test_version_detection_ctx() {
        let parser = AlsParser::new();
        let doc = parser.parse("!ctx\n#col\n1").unwrap();
        assert!(doc.is_ctx());
    }

    #[test]
    fn test_version_detection_no_version() {
        // When no version is specified, default to v1 ALS
        let parser = AlsParser::new();
        let doc = parser.parse("#col\n1").unwrap();
        assert_eq!(doc.version, 1);
        assert!(doc.is_als());
    }

    #[test]
    fn test_version_future_version_error() {
        let parser = AlsParser::new();
        let result = parser.parse("!v2\n#col\n1");
        assert!(matches!(result, Err(AlsError::VersionMismatch { expected: 1, found: 2 })));
    }

    #[test]
    fn test_version_very_high_version_error() {
        let parser = AlsParser::new();
        let result = parser.parse("!v255\n#col\n1");
        assert!(matches!(result, Err(AlsError::VersionMismatch { expected: 1, found: 255 })));
    }
}
