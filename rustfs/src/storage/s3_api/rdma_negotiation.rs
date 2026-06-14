// Copyright 2024 RustFS Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use http::HeaderMap;
use metrics::counter;
use s3s::{S3Error, S3ErrorCode, S3Result};

pub(crate) const S3_RDMA_TOKEN_HEADER: &str = "x-amz-rdma-token";
pub(crate) const S3_RDMA_REPLY_HEADER: &str = "x-amz-rdma-reply";
pub(crate) const S3_RDMA_UNSUPPORTED_MESSAGE: &str = "S3 RDMA data path is not available in this build";
pub(crate) const S3_RDMA_GET_OBJECT_OPERATION: &str = "s3:GetObject";
pub(crate) const S3_RDMA_RANGE_GET_OPERATION: &str = "s3:RangeGetObject";

const S3_RDMA_NEGOTIATION_REQUESTS_TOTAL: &str = "rustfs_system_network_s3_rdma_negotiation_requests_total";
const OPERATION_LABEL: &str = "operation";
const CLASSIFICATION_LABEL: &str = "classification";
const REQUESTED_UNSUPPORTED_CLASSIFICATION: &str = "requested_unsupported";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum S3RdmaNegotiation {
    NotRequested,
    RequestedUnsupported,
}

pub(crate) fn detect_s3_rdma_negotiation(headers: &HeaderMap) -> S3RdmaNegotiation {
    if headers.contains_key(S3_RDMA_TOKEN_HEADER) {
        S3RdmaNegotiation::RequestedUnsupported
    } else {
        S3RdmaNegotiation::NotRequested
    }
}

pub(crate) fn get_object_rdma_negotiation_operation(has_range: bool) -> &'static str {
    if has_range {
        S3_RDMA_RANGE_GET_OPERATION
    } else {
        S3_RDMA_GET_OBJECT_OPERATION
    }
}

pub(crate) fn reject_unsupported_s3_rdma_negotiation(headers: &HeaderMap, operation: &'static str) -> S3Result<()> {
    match detect_s3_rdma_negotiation(headers) {
        S3RdmaNegotiation::NotRequested => Ok(()),
        S3RdmaNegotiation::RequestedUnsupported => {
            counter!(
                S3_RDMA_NEGOTIATION_REQUESTS_TOTAL,
                OPERATION_LABEL => operation,
                CLASSIFICATION_LABEL => REQUESTED_UNSUPPORTED_CLASSIFICATION
            )
            .increment(1);
            Err(S3Error::with_message(S3ErrorCode::InvalidRequest, S3_RDMA_UNSUPPORTED_MESSAGE))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{HeaderMap, HeaderValue};

    #[test]
    fn negotiation_is_not_requested_without_rdma_token() {
        let headers = HeaderMap::new();

        assert_eq!(detect_s3_rdma_negotiation(&headers), S3RdmaNegotiation::NotRequested);
    }

    #[test]
    fn negotiation_fails_closed_when_rdma_token_is_present() {
        let mut headers = HeaderMap::new();
        headers.insert(S3_RDMA_TOKEN_HEADER, HeaderValue::from_static("client-token"));

        assert_eq!(detect_s3_rdma_negotiation(&headers), S3RdmaNegotiation::RequestedUnsupported);
    }

    #[test]
    fn negotiation_header_names_are_stable() {
        assert_eq!(S3_RDMA_TOKEN_HEADER, "x-amz-rdma-token");
        assert_eq!(S3_RDMA_REPLY_HEADER, "x-amz-rdma-reply");
    }

    #[test]
    fn get_object_negotiation_operation_distinguishes_range_get() {
        assert_eq!(get_object_rdma_negotiation_operation(false), "s3:GetObject");
        assert_eq!(get_object_rdma_negotiation_operation(true), "s3:RangeGetObject");
    }

    #[test]
    fn unsupported_negotiation_rejection_is_noop_without_rdma_token() {
        let headers = HeaderMap::new();

        reject_unsupported_s3_rdma_negotiation(&headers, "s3:GetObject").expect("request should continue without token");
    }

    #[test]
    fn unsupported_negotiation_rejection_returns_invalid_request() {
        let mut headers = HeaderMap::new();
        headers.insert(S3_RDMA_TOKEN_HEADER, HeaderValue::from_static("client-token"));

        let err = reject_unsupported_s3_rdma_negotiation(&headers, "s3:GetObject").unwrap_err();

        assert_eq!(err.code(), &S3ErrorCode::InvalidRequest);
        assert_eq!(err.message(), Some(S3_RDMA_UNSUPPORTED_MESSAGE));
    }
}
