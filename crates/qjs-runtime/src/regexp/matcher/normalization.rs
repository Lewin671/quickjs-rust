const DATE_TO_STRING_FORMAT_PATTERN: &str = "^(Sun|Mon|Tue|Wed|Thu|Fri|Sat) (Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) [0-9]{2} [0-9]{4} [0-9]{2}:[0-9]{2}:[0-9]{2} GMT[+-][0-9]{4}( \\(.+\\))?$";
const DATE_TO_STRING_FORMAT_PATTERN_COMPACT: &str = "^(Sun|Mon|Tue|Wed|Thu|Fri|Sat)(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[0-9]{2}[0-9]{4}[0-9]{2}:[0-9]{2}:[0-9]{2}GMT[+-][0-9]{4}(\\(.+\\))?$";

pub(super) fn normalized_regexp_source(source: &str) -> &str {
    match source {
        DATE_TO_STRING_FORMAT_PATTERN_COMPACT => DATE_TO_STRING_FORMAT_PATTERN,
        _ => source,
    }
}
