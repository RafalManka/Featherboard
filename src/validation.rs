pub const TITLE_MIN: usize = 3;
pub const TITLE_MAX: usize = 200;
pub const DESCRIPTION_MIN: usize = 10;
pub const DESCRIPTION_MAX: usize = 5000;
pub const AUTHOR_NAME_MIN: usize = 1;
pub const AUTHOR_NAME_MAX: usize = 80;
pub const COMMENT_BODY_MIN: usize = 1;
pub const COMMENT_BODY_MAX: usize = 2000;

pub fn validate_len(value: &str, field: &str, min: usize, max: usize) -> Result<(), String> {
    let len = value.chars().count();
    if len < min {
        return Err(format!("{field} must be at least {min} characters"));
    }
    if len > max {
        return Err(format!("{field} must be at most {max} characters"));
    }
    Ok(())
}
