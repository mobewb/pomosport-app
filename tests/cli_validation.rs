use pomosport::config::{parse_exercises, parse_minutes};

#[test]
fn exercises_parser_trims_and_bounds() {
    assert_eq!(parse_exercises(" 5 a , ,b ").unwrap(), ["5 a", "b"]);
    assert!(parse_exercises(" , ").is_err());
    assert!(parse_exercises("1,2,3,4,5,6,7,8,9").is_err());
}

#[test]
fn minutes_parser_accepts_only_sane_values() {
    for ok in ["25", "0.1", "1440"] {
        assert!(parse_minutes(ok).is_ok(), "{ok}");
    }
    for bad in ["0", "-1", "NaN", "inf", "1441", "abc", ""] {
        assert!(parse_minutes(bad).is_err(), "{bad}");
    }
}
