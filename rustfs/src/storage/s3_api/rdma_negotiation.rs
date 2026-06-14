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

pub(crate) const S3_RDMA_TOKEN_HEADER: &str = "x-amz-rdma-token";
pub(crate) const S3_RDMA_REPLY_HEADER: &str = "x-amz-rdma-reply";

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
}
