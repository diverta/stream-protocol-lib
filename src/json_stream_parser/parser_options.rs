#[derive(Default)]
pub struct ParserOptions {
    pub filter: ParserOptionsFilter,
}

#[derive(Default)]
pub struct ParserOptionsFilter {
    pub output_whitelist: Option<Vec<String>>, // An optional list of json paths to apply for a whitelist of the output data
    pub buffer_whitelist: Option<Vec<String>>, // An optional list of json paths to apply for a whitelist of the buffered data
}

impl ParserOptions {
    pub fn new_with_filter(filter: ParserOptionsFilter) -> Self {
        ParserOptions {
            filter
        }
    }

    pub fn new_with_filter_output_whitelist(output_whitelist: Option<Vec<String>>) -> Self {
        ParserOptions {
            filter: ParserOptionsFilter {
                output_whitelist,
                buffer_whitelist: None,
            }
        }
    }

    pub fn new_with_filter_buffer_whitelist(buffer_whitelist: Option<Vec<String>>) -> Self {
        ParserOptions {
            filter: ParserOptionsFilter {
                output_whitelist: None,
                buffer_whitelist,
            }
        }
    }

    pub fn new_with_filter_output_buffer_whitelist(
        output_whitelist: Option<Vec<String>>,
        buffer_whitelist: Option<Vec<String>>
    ) -> Self {
        ParserOptions {
            filter: ParserOptionsFilter {
                output_whitelist,
                buffer_whitelist,
            }
        }
    }
}