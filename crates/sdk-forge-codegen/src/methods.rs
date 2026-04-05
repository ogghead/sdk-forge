//! Per-endpoint method metadata generation.
//!
//! Builds [`EndpointMethod`](crate::types::EndpointMethod) values from
//! an [`ApiModel`](sdk_forge_session::types::ApiModel) and a
//! [`TypeRegistry`](crate::types::TypeRegistry). This module is thin
//! because the heavy lifting happens in [`crate::types::resolve_all`].

// Endpoint method generation is handled directly by `types::resolve_all`.
// This module exists as a namespace for future pagination-aware helpers.

use sdk_forge_session::types::PaginationPattern;
use serde::Serialize;

use crate::types::EndpointMethod;

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

/// Filter methods that look like list operations (GET returning `Vec<>`).
pub fn list_methods(methods: &[EndpointMethod]) -> Vec<&EndpointMethod> {
    methods
        .iter()
        .filter(|m| m.http_method == "get" && m.response_type.starts_with("Vec<"))
        .collect()
}

#[cfg(test)]
mod tests {
    use sdk_forge_session::types::PaginationPattern;

    use super::{build_pagination_info, list_methods};
    use crate::types::EndpointMethod;

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
    fn test_list_methods_filter() {
        let methods = vec![
            EndpointMethod {
                fn_name: "list_users".to_owned(),
                doc_comment: String::new(),
                http_method: "get".to_owned(),
                path_format_str: String::new(),
                path_params: Vec::new(),
                query_params: Vec::new(),
                request_body_type: None,
                response_type: "Vec<User>".to_owned(),
                requires_auth: false,
            },
            EndpointMethod {
                fn_name: "get_user".to_owned(),
                doc_comment: String::new(),
                http_method: "get".to_owned(),
                path_format_str: String::new(),
                path_params: Vec::new(),
                query_params: Vec::new(),
                request_body_type: None,
                response_type: "User".to_owned(),
                requires_auth: false,
            },
            EndpointMethod {
                fn_name: "create_user".to_owned(),
                doc_comment: String::new(),
                http_method: "post".to_owned(),
                path_format_str: String::new(),
                path_params: Vec::new(),
                query_params: Vec::new(),
                request_body_type: Some("CreateUserRequest".to_owned()),
                response_type: "User".to_owned(),
                requires_auth: false,
            },
        ];

        let lists = list_methods(&methods);
        assert_eq!(lists.len(), 1, "only list_users returns Vec<>");
        assert_eq!(lists.first().unwrap().fn_name, "list_users");
    }
}
