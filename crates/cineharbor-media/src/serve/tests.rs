use super::*;
use std::net::SocketAddr;
use url::Url;

async fn serve_router(router: Router) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    addr
}

fn fixture_parts(addr: SocketAddr) -> Arc<ProxyParts> {
    Arc::new(ProxyParts {
        client: MediaClient::for_fixture(addr),
        sources: Arc::new(
            [
                ("src".into(), SourceHeaders::default()),
                ("live1".into(), SourceHeaders::default()),
            ]
            .into(),
        ),
        public_base_url: "http://proxy.test".into(),
        authorization: Arc::new(
            MediaAuthorization::new(b"0123456789abcdef0123456789abcdef").unwrap(),
        ),
    })
}

fn request_url(
    addr: SocketAddr,
    parts: &ProxyParts,
    kind: &str,
    asset: &str,
    source: &str,
    target: &str,
) -> Url {
    let path = format!("/media/{kind}/{asset}");
    let uri = if kind == "vod" {
        crate::build_vod_proxy_url(&parts.public_base_url, &path, source, target)
    } else {
        crate::build_live_proxy_url(&parts.public_base_url, &path, source, target, false)
    };
    let signed = parts
        .authorization
        .sign_proxy_url(&uri, &parts.public_base_url)
        .unwrap();
    localize(&signed, addr)
}

fn localize(uri: &str, addr: SocketAddr) -> Url {
    let mut parsed = Url::parse(uri).unwrap();
    parsed.set_host(Some(&addr.ip().to_string())).unwrap();
    parsed.set_port(Some(addr.port())).unwrap();
    parsed
}

fn assert_capability(uri: &str, parts: &ProxyParts) {
    let parsed = Url::parse(uri).unwrap();
    let pairs = parsed.query_pairs().collect::<HashMap<_, _>>();
    let scope = parsed.path().strip_prefix("/media/").unwrap();
    let source_key = if scope.starts_with("vod/") {
        "source"
    } else {
        "cineharbor-source"
    };
    assert!(parts.authorization.verify(
        &parts.public_base_url,
        scope,
        &pairs[source_key],
        &pairs["url"],
        pairs["expires"].parse().unwrap(),
        &pairs["sig"]
    ));
    assert!(!pairs.contains_key("token"));
}

#[tokio::test]
async fn vod_m3u8_proxies_and_rewrites() {
    let upstream = serve_router(Router::new().route(
        "/p/index.m3u8",
        get(|| async {
            "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\n#EXTINF:4,\nseg1.ts\n"
        }),
    ))
    .await;
    let parts = fixture_parts(upstream);
    let proxy = serve_router(vod_proxy_router(parts.clone())).await;
    let response = reqwest::get(request_url(
        proxy,
        &parts,
        "vod",
        "m3u8",
        "src",
        &format!("http://{upstream}/p/index.m3u8"),
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/vnd.apple.mpegurl"
    );
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let body = response.text().await.unwrap();
    assert!(body.contains("http://proxy.test/media/vod/key?source=src&url="));
    assert!(body.contains("http://proxy.test/media/vod/segment?source=src&url="));
    assert!(body.contains("seg1.ts"));
}

#[tokio::test]
async fn vod_proxy_requires_token_when_configured() {
    // The old optional bearer-token boundary is now mandatory resource authorization.
    let parts = fixture_parts("127.0.0.1:9".parse().unwrap());
    let proxy = serve_router(vod_proxy_router(parts)).await;
    let response = reqwest::get(format!(
        "http://{proxy}/media/vod/segment?source=src&url=http://x/a.ts"
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn live_m3u8_uses_cineharbor_source_param() {
    let upstream = serve_router(Router::new().route(
        "/live/master.m3u8",
        get(|| async { "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000\nch.m3u8\n" }),
    ))
    .await;
    let parts = fixture_parts(upstream);
    let proxy = serve_router(live_proxy_router(parts.clone())).await;
    let response = reqwest::get(request_url(
        proxy,
        &parts,
        "live",
        "m3u8",
        "live1",
        &format!("http://{upstream}/live/master.m3u8"),
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    assert!(body.contains("http://proxy.test/media/live/m3u8?cineharbor-source=live1&url="));
    let child = body.lines().find(|line| line.starts_with("http:")).unwrap();
    assert_capability(child, &parts);
}

#[tokio::test]
async fn redirected_manifests_use_the_final_document_for_vod_and_live() {
    let upstream = serve_router(
        Router::new()
            .route(
                "/original.m3u8",
                get(|| async { axum::response::Redirect::temporary("/cdn/current/index.m3u8") }),
            )
            .route(
                "/cdn/current/index.m3u8",
                get(|| async { "#EXTM3U\n#EXTINF:4,\nsegment.ts\n" }),
            ),
    )
    .await;
    let parts = fixture_parts(upstream);
    let proxy =
        serve_router(vod_proxy_router(parts.clone()).merge(live_proxy_router(parts.clone()))).await;
    for kind in ["vod", "live"] {
        let response = reqwest::get(request_url(
            proxy,
            &parts,
            kind,
            "m3u8",
            "src",
            &format!("http://{upstream}/original.m3u8"),
        ))
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let text = response.text().await.unwrap();
        let uri = text.lines().find(|line| line.starts_with("http:")).unwrap();
        assert_capability(uri, &parts);
        let parsed = Url::parse(uri).unwrap();
        let target = parsed
            .query_pairs()
            .find(|(key, _)| key == "url")
            .unwrap()
            .1;
        assert_eq!(target, format!("http://{upstream}/cdn/current/segment.ts"));
    }
}

#[tokio::test]
async fn authorized_multitrack_manifest_keeps_tokens_on_resource_uris() {
    let upstream = serve_router(Router::new().route("/index.m3u8", get(|| async { "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,URI=\"audio.m3u8\",NAME=\"English\"\n#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\",KEYFORMAT=\"identity\"\n#EXTINF:4,\nsegment.ts\n" }))).await;
    let parts = fixture_parts(upstream);
    let proxy =
        serve_router(vod_proxy_router(parts.clone()).merge(live_proxy_router(parts.clone()))).await;
    let regex = regex::Regex::new(r#"URI="([^"]+)""#).unwrap();
    for kind in ["vod", "live"] {
        let response = reqwest::get(request_url(
            proxy,
            &parts,
            kind,
            "m3u8",
            "src",
            &format!("http://{upstream}/index.m3u8"),
        ))
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let text = response.text().await.unwrap();
        assert!(text.contains("NAME=\"English\""));
        assert!(text.contains("KEYFORMAT=\"identity\""));
        let mut resources = regex
            .captures_iter(&text)
            .map(|capture| capture[1].to_string())
            .collect::<Vec<_>>();
        resources.extend(
            text.lines()
                .filter(|line| line.starts_with("http:"))
                .map(str::to_string),
        );
        assert_eq!(resources.len(), 3);
        for uri in resources {
            assert_capability(&uri, &parts);
        }
    }
}

#[tokio::test]
async fn upstream_failures_do_not_disclose_signed_urls() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let parts = fixture_parts(addr);
    let url = format!("http://{addr}/private.m3u8?signature=must-not-leak");
    let manifest = fetch_manifest(&parts, "src", &url).await.unwrap_err();
    let bytes = forward_bytes(&parts, "src", &url, Method::GET, &HeaderMap::new(), 1024).await;
    for response in [*manifest, bytes] {
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(&body[..], b"upstream fetch failed");
    }
}

#[tokio::test]
async fn every_route_rejects_missing_or_tampered_capabilities_before_egress() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let hits = Arc::new(AtomicUsize::new(0));
    let record = hits.clone();
    let upstream = serve_router(Router::new().route(
        "/data",
        get(move || {
            let record = record.clone();
            async move {
                record.fetch_add(1, Ordering::SeqCst);
                "#EXTM3U\n"
            }
        }),
    ))
    .await;
    let parts = fixture_parts(upstream);
    let proxy =
        serve_router(vod_proxy_router(parts.clone()).merge(live_proxy_router(parts.clone()))).await;
    for kind in ["vod", "live"] {
        for asset in ["m3u8", "segment", "key"] {
            let signed = request_url(
                proxy,
                &parts,
                kind,
                asset,
                "src",
                &format!("http://{upstream}/data"),
            );
            for field in ["sig", "expires", "url"] {
                let mut altered = signed.clone();
                let pairs = altered
                    .query_pairs()
                    .into_owned()
                    .filter(|(key, _)| key != field)
                    .collect::<Vec<_>>();
                altered.query_pairs_mut().clear().extend_pairs(pairs);
                if field == "url" {
                    altered
                        .query_pairs_mut()
                        .append_pair("url", &format!("http://{upstream}/other"));
                }
                assert_eq!(
                    reqwest::get(altered).await.unwrap().status(),
                    StatusCode::UNAUTHORIZED
                );
            }
            let mut changed_path = signed.clone();
            changed_path.set_path(&format!(
                "/media/{kind}/{}",
                if asset == "key" { "segment" } else { "key" }
            ));
            assert_eq!(
                reqwest::get(changed_path).await.unwrap().status(),
                StatusCode::UNAUTHORIZED
            );
            let unknown = request_url(
                proxy,
                &parts,
                kind,
                asset,
                "unknown",
                &format!("http://{upstream}/data"),
            );
            assert_eq!(
                reqwest::get(unknown).await.unwrap().status(),
                StatusCode::UNAUTHORIZED
            );
            let mut duplicate = signed.clone();
            duplicate
                .query_pairs_mut()
                .append_pair("url", "http://127.0.0.1/private");
            assert_eq!(
                reqwest::get(duplicate).await.unwrap().status(),
                StatusCode::BAD_REQUEST
            );
        }
    }
    assert_eq!(hits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn redirect_cannot_reach_another_private_origin_and_loops_are_bounded() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let hits = Arc::new(AtomicUsize::new(0));
    let record = hits.clone();
    let protected = serve_router(Router::new().route(
        "/secret",
        get(move || {
            let record = record.clone();
            async move {
                record.fetch_add(1, Ordering::SeqCst);
                "must-not-read"
            }
        }),
    ))
    .await;
    let private = format!("http://{protected}/secret");
    let upstream = serve_router(
        Router::new()
            .route(
                "/redirect",
                get(move || {
                    let private = private.clone();
                    async move { axum::response::Redirect::temporary(&private) }
                }),
            )
            .route(
                "/loop",
                get(|| async { axum::response::Redirect::temporary("/loop") }),
            ),
    )
    .await;
    let parts = fixture_parts(upstream);
    let denied = forward_bytes(
        &parts,
        "src",
        &format!("http://{upstream}/redirect"),
        Method::GET,
        &HeaderMap::new(),
        1024,
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    let looped = forward_bytes(
        &parts,
        "src",
        &format!("http://{upstream}/loop"),
        Method::GET,
        &HeaderMap::new(),
        1024,
    )
    .await;
    assert_eq!(looped.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn ranges_head_and_cors_preserve_byte_semantics_without_forwarding_credentials() {
    use std::sync::Mutex;
    let observed = Arc::new(Mutex::new(Vec::new()));
    let record = observed.clone();
    let upstream = serve_router(Router::new().route(
        "/data",
        get(move |method: Method, headers: HeaderMap| {
            let record = record.clone();
            async move {
                record
                    .lock()
                    .unwrap()
                    .push((method.clone(), headers.clone()));
                let mut response = if method == Method::HEAD {
                    (StatusCode::OK, "abcdef").into_response()
                } else if headers.contains_key(header::RANGE) {
                    (StatusCode::PARTIAL_CONTENT, "cde").into_response()
                } else {
                    (StatusCode::OK, "abcdef").into_response()
                };
                response
                    .headers_mut()
                    .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
                response.headers_mut().insert(
                    header::CONTENT_LENGTH,
                    HeaderValue::from(
                        if method == Method::HEAD || !headers.contains_key(header::RANGE) {
                            6
                        } else {
                            3
                        },
                    ),
                );
                response
                    .headers_mut()
                    .insert(header::ETAG, HeaderValue::from_static("\"test\""));
                response.headers_mut().insert(
                    header::SET_COOKIE,
                    HeaderValue::from_static("session=private"),
                );
                if response.status() == StatusCode::PARTIAL_CONTENT {
                    response.headers_mut().insert(
                        header::CONTENT_RANGE,
                        HeaderValue::from_static("bytes 2-4/6"),
                    );
                }
                response
            }
        }),
    ))
    .await;
    let parts = fixture_parts(upstream);
    let proxy =
        serve_router(vod_proxy_router(parts.clone()).merge(live_proxy_router(parts.clone()))).await;
    for kind in ["vod", "live"] {
        let request = request_url(
            proxy,
            &parts,
            kind,
            "segment",
            "src",
            &format!("http://{upstream}/data"),
        );
        let response = reqwest::Client::new()
            .get(request.clone())
            .header(header::RANGE, "bytes=2-4")
            .header(header::IF_RANGE, "\"test\"")
            .header(header::AUTHORIZATION, "Bearer never-forward")
            .header(header::COOKIE, "secret=yes")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes 2-4/6");
        assert_eq!(response.headers()[header::CONTENT_LENGTH], "3");
        assert_eq!(response.headers()[header::ETAG], "\"test\"");
        assert!(!response.headers().contains_key(header::SET_COOKIE));
        assert_eq!(response.text().await.unwrap(), "cde");
        let head = reqwest::Client::new()
            .head(request.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(head.status(), StatusCode::OK);
        assert_eq!(head.headers()[header::CONTENT_LENGTH], "6");
        assert!(head.bytes().await.unwrap().is_empty());
        let options = reqwest::Client::new()
            .request(Method::OPTIONS, request)
            .header(header::ORIGIN, "https://web.test")
            .send()
            .await
            .unwrap();
        assert_eq!(options.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            options.headers()[header::ACCESS_CONTROL_ALLOW_HEADERS],
            "Range, If-Range"
        );
    }
    let recorded = observed.lock().unwrap();
    assert_eq!(recorded.len(), 4);
    for (method, headers) in recorded.iter() {
        assert!(!headers.contains_key(header::COOKIE));
        assert!(!headers.contains_key(header::AUTHORIZATION));
        assert_eq!(headers[header::ACCEPT_ENCODING], "identity");
        if *method == Method::GET {
            assert_eq!(headers[header::IF_RANGE], "\"test\"");
        }
    }
    assert_eq!(
        recorded
            .iter()
            .filter(|(method, _)| *method == Method::HEAD)
            .count(),
        2
    );
}

#[test]
fn range_syntax_is_single_bounded_and_unambiguous() {
    for value in ["bytes=0-99", "bytes=10-", "bytes=-10"] {
        assert!(valid_range(value));
    }
    for value in [
        "",
        "bytes=-",
        "bytes=-0",
        "bytes=5-2",
        "bytes=0-1,2-3",
        "bytes=+1-3",
        "bytes=0-18446744073709551616",
        "items=0-1",
        "bytes=1-+3",
    ] {
        assert!(!valid_range(value), "{value}");
    }
}

#[tokio::test]
async fn oversized_unknown_length_streams_are_terminated_and_known_sizes_are_rejected() {
    let upstream = serve_router(
        Router::new()
            .route(
                "/chunked",
                get(|| async {
                    Body::from_stream(futures_util::stream::iter([
                        Ok::<_, std::io::Error>("abcd"),
                        Ok("efgh"),
                    ]))
                }),
            )
            .route("/known", get(|| async { "12345678" }))
            .route(
                "/range",
                get(|| async {
                    (
                        StatusCode::RANGE_NOT_SATISFIABLE,
                        [(header::CONTENT_RANGE, "bytes */8")],
                        "upstream-private-detail",
                    )
                }),
            ),
    )
    .await;
    let parts = fixture_parts(upstream);
    let chunked = forward_bytes(
        &parts,
        "src",
        &format!("http://{upstream}/chunked"),
        Method::GET,
        &HeaderMap::new(),
        5,
    )
    .await;
    assert_eq!(chunked.status(), StatusCode::OK);
    assert!(
        axum::body::to_bytes(chunked.into_body(), 100)
            .await
            .is_err()
    );
    let known = forward_bytes(
        &parts,
        "src",
        &format!("http://{upstream}/known"),
        Method::GET,
        &HeaderMap::new(),
        5,
    )
    .await;
    assert_eq!(known.status(), StatusCode::BAD_GATEWAY);
    let range = forward_bytes(
        &parts,
        "src",
        &format!("http://{upstream}/range"),
        Method::GET,
        &HeaderMap::new(),
        5,
    )
    .await;
    assert_eq!(range.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(range.headers()[header::CONTENT_RANGE], "bytes */8");
    assert!(
        axum::body::to_bytes(range.into_body(), 100)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn rejects_oversized_keys_and_invalid_or_oversized_manifests() {
    let upstream = serve_router(
        Router::new()
            .route(
                "/large",
                get(|| async { "X".repeat(MAX_MANIFEST_BYTES + 1) }),
            )
            .route("/key", get(|| async { "X".repeat(MAX_KEY_BYTES + 1) }))
            .route("/not-hls", get(|| async { "<html>not a playlist</html>" }))
            .route(
                "/compressed",
                get(|| async { ([(header::CONTENT_ENCODING, "gzip")], "#EXTM3U\n") }),
            ),
    )
    .await;
    let parts = fixture_parts(upstream);
    for path in ["large", "not-hls", "compressed"] {
        let response = fetch_manifest(&parts, "src", &format!("http://{upstream}/{path}"))
            .await
            .unwrap_err();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }
    let proxy = serve_router(vod_proxy_router(parts.clone())).await;
    let response = reqwest::get(request_url(
        proxy,
        &parts,
        "vod",
        "key",
        "src",
        &format!("http://{upstream}/key"),
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn manifest_head_never_advertises_the_unrewritten_length_or_fetches_a_body() {
    let upstream = serve_router(
        Router::new().route(
            "/list",
            get(|| async { StatusCode::IM_A_TEAPOT })
                .head(|| async { ([(header::CONTENT_LENGTH, "123")], "") }),
        ),
    )
    .await;
    let parts = fixture_parts(upstream);
    let proxy = serve_router(vod_proxy_router(parts.clone())).await;
    let url = request_url(
        proxy,
        &parts,
        "vod",
        "m3u8",
        "src",
        &format!("http://{upstream}/list"),
    );
    let response = reqwest::Client::new().head(url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!response.headers().contains_key(header::CONTENT_LENGTH));
    assert!(response.bytes().await.unwrap().is_empty());
}

#[test]
fn content_ranges_reject_malformed_inconsistent_or_overflowed_extents() {
    for value in ["bytes 1-2/3", "bytes 1-2/*"] {
        assert_eq!(range_extent(value, StatusCode::PARTIAL_CONTENT), Some(2));
    }
    assert_eq!(
        range_extent("bytes */0", StatusCode::RANGE_NOT_SATISFIABLE),
        Some(0)
    );
    for value in [
        "bytes 2-1/3",
        "bytes 1-3/3",
        "bytes */3",
        "bytes 0-18446744073709551615/*",
        "bytes 0-1/-2",
        "bytes 0-1/+2",
        "items 0-1/2",
    ] {
        assert!(
            range_extent(value, StatusCode::PARTIAL_CONTENT).is_none(),
            "{value}"
        );
    }
}

#[tokio::test]
async fn manifest_expansion_is_bounded_before_signing() {
    let upstream = serve_router(Router::new().route(
        "/list",
        get(|| async { format!("#EXTM3U\n{}", "#EXTINF:4,\na.ts\n".repeat(20_001)) }),
    ))
    .await;
    let parts = fixture_parts(upstream);
    let proxy = serve_router(vod_proxy_router(parts.clone())).await;
    let url = request_url(
        proxy,
        &parts,
        "vod",
        "m3u8",
        "src",
        &format!("http://{upstream}/list"),
    );
    assert_eq!(
        reqwest::get(url).await.unwrap().status(),
        StatusCode::BAD_GATEWAY
    );
}

#[tokio::test]
async fn upstream_slot_lives_until_the_downstream_body_is_dropped() {
    let upstream = serve_router(Router::new().route("/data", get(|| async { "data" }))).await;
    let parts = fixture_parts(upstream);
    assert_eq!(parts.client.available_slots(), 16);
    let response = forward_bytes(
        &parts,
        "src",
        &format!("http://{upstream}/data"),
        Method::GET,
        &HeaderMap::new(),
        1024,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(parts.client.available_slots(), 15);
    drop(response);
    assert_eq!(parts.client.available_slots(), 16);
}
