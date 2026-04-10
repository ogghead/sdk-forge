//! Per-endpoint method metadata and pagination helpers.
//!
//! Most endpoint method generation is handled by [`crate::types::resolve_all`].
//! This module provides pagination-aware helpers and filtering utilities.

use sdk_forge_session::types::PaginationPattern;
use serde::Serialize;

use crate::types::WitFunction;

/// Pagination metadata for template rendering.
#[derive(Debug, Clone, Serialize)]
pub struct PaginationInfo {
    /// The pagination style.
    pub style: String,
    /// Parameter names relevant to this pattern.
    pub params: Vec<String>,
}

/// Build pagination info from a detected pattern.
pub fn build_pagination_info(pattern: &Option<PaginationPattern>) -> Option<PaginationInfo> {
    let pat = pattern.as_ref()?;

    let (style, params) = match pat {
        PaginationPattern::Cursor { cursor_param } => {
            ("cursor".to_owned(), vec![cursor_param.clone()])
        }
        PaginationPattern::OffsetLimit {
            offset_param,
            limit_param,
        } => (
            "offset_limit".to_owned(),
            vec![offset_param.clone(), limit_param.clone()],
        ),
        PaginationPattern::PageNumber { page_param } => {
            ("page_number".to_owned(), vec![page_param.clone()])
        }
        PaginationPattern::LinkHeader => ("link_header".to_owned(), Vec::new()),
    };

    Some(PaginationInfo { style, params })
}

/// Filter functions that look like list operations (GET returning `list<>`).
pub fn list_functions(functions: &[WitFunction]) -> Vec<&WitFunction> {
    functions
        .iter()
        .filter(|f| f.http_method == "get" && f.response_rust.starts_with("Vec<"))
        .collect()
}

#[cfg(test)]
mod tests {
    use sdk_forge_session::types::PaginationPattern;

    use super::{build_pagination_info, list_functions};
    use crate::types::WitFunction;

    #[test]
    fn test_pagination_cursor() {
        let info = build_pagination_info(&Some(PaginationPattern::Cursor {
            cursor_param: "after".to_owned(),
        }));
        assert!(info.is_some(), "cursor pagination should produce info");

        let info_val = info.unwrap_or_else(|| unreachable!());
        assert_eq!(info_val.style, "cursor");
        assert_eq!(info_val.params, vec!["after"]);
    }

    #[test]
    fn test_pagination_offset_limit() {
        let info = build_pagination_info(&Some(PaginationPattern::OffsetLimit {
            offset_param: "skip".to_owned(),
            limit_param: "take".to_owned(),
        }));
        assert!(info.is_some());

        let info_val = info.unwrap_or_else(|| unreachable!());
        assert_eq!(info_val.style, "offset_limit");
        assert_eq!(info_val.params, vec!["skip", "take"]);
    }

    #[test]
    fn test_pagination_none() {
        let info = build_pagination_info(&None);
        assert!(info.is_none(), "no pattern means no info");
    }

    #[test]
    fn test_list_functions_filter() {
        let functions = vec![
            WitFunction {
                wit_name: "list-users".to_owned(),
                rust_name: "list_users".to_owned(),
                doc_comment: String::new(),
                http_method: "get".to_owned(),
                path_format_str: String::new(),
                path_params: Vec::new(),
                query_params: Vec::new(),
                request_body_wit: None,
                request_body_rust: None,
                response_wit: "list<user>".to_owned(),
                response_rust: "Vec<User>".to_owned(),
                requires_auth: false,
            },
            WitFunction {
                wit_name: "get-user".to_owned(),
                rust_name: "get_user".to_owned(),
                doc_comment: String::new(),
                http_method: "get".to_owned(),
                path_format_str: String::new(),
                path_params: Vec::new(),
                query_params: Vec::new(),
                request_body_wit: None,
                request_body_rust: None,
                response_wit: "user".to_owned(),
                response_rust: "User".to_owned(),
                requires_auth: false,
            },
            WitFunction {
                wit_name: "create-user".to_owned(),
                rust_name: "create_user".to_owned(),
                doc_comment: String::new(),
                http_method: "post".to_owned(),
                path_format_str: String::new(),
                path_params: Vec::new(),
                query_params: Vec::new(),
                request_body_wit: Some("create-user-request".to_owned()),
                request_body_rust: Some("CreateUserRequest".to_owned()),
                response_wit: "user".to_owned(),
                response_rust: "User".to_owned(),
                requires_auth: false,
            },
        ];

        let lists = list_functions(&functions);
        assert_eq!(lists.len(), 1, "only list_users returns Vec<>");
        assert_eq!(lists.first().unwrap().rust_name, "list_users");
    }
}
