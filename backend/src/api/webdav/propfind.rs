use axum::{
    body::{to_bytes, Body},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use metrics::histogram;
use quick_xml::{events::Event, reader::Reader};
use std::{collections::HashMap, time::Instant};
use uuid::Uuid;

use super::{
    lock::{active_lock_discoveries, LockDiscoveryByPath},
    path::{dav_href, lock_key},
    webdav_error_response,
    xml_fragments::{multistatus_start, xml_element, xml_empty, xml_end, xml_text},
};
use crate::{
    services::webdav::{WebDavError, WebDavPrincipal, WebDavService},
    AppState,
};

pub(super) async fn propfind(
    state: &AppState,
    principal: &WebDavPrincipal,
    segments: &[String],
    headers: &HeaderMap,
    body: Body,
) -> Response {
    let start = Instant::now();
    let depth = headers
        .get("depth")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("1");
    let requested = match requested_propfind_props(body).await {
        Ok(requested) => requested,
        Err(status) => return status.into_response(),
    };
    let folder_id = match WebDavService::new(state)
        .resolve_parent(principal, segments)
        .await
    {
        Ok(folder_id) => folder_id,
        Err(WebDavError::NotFound) => {
            return propfind_file(state, principal, segments, &requested).await
        }
        Err(error) => return webdav_error_response(error),
    };
    let lock_discoveries =
        if requested.props.contains(&DavProp::LockDiscovery) && !requested.propname {
            match active_lock_discoveries(state, principal.user_id).await {
                Ok(lock_discoveries) => lock_discoveries,
                Err(status) => return status.into_response(),
            }
        } else {
            HashMap::new()
        };
    let mut xml = multistatus_start();
    xml.push_str(&collection_response(
        segments,
        None,
        &requested,
        None,
        &lock_discoveries,
    ));
    if depth != "0" {
        let mut count = 1usize;
        let recursive = depth.eq_ignore_ascii_case("infinity");
        let mut walk = PropfindWalk {
            requested: &requested,
            recursive,
            lock_discoveries: &lock_discoveries,
            xml: &mut xml,
            count: &mut count,
        };
        if let Err(status) =
            append_propfind_children(state, principal, folder_id, segments, &mut walk).await
        {
            return status.into_response();
        }
    }
    xml.push_str(&xml_end("D:multistatus"));
    histogram!("webdav_propfind_duration_seconds").record(start.elapsed().as_secs_f64());
    (
        StatusCode::MULTI_STATUS,
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

const MAX_PROPFIND_INFINITY_ITEMS: usize = 5_000;

struct PropfindWalk<'a> {
    requested: &'a PropfindRequest,
    recursive: bool,
    lock_discoveries: &'a LockDiscoveryByPath,
    xml: &'a mut String,
    count: &'a mut usize,
}

async fn append_propfind_children(
    state: &AppState,
    principal: &WebDavPrincipal,
    parent_id: Option<Uuid>,
    parent_segments: &[String],
    walk: &mut PropfindWalk<'_>,
) -> Result<(), StatusCode> {
    let service = WebDavService::new(state);
    let mut pending = vec![(parent_id, parent_segments.to_vec())];

    while let Some((current_parent_id, current_segments)) = pending.pop() {
        let children = service
            .list_children(principal.user_id, current_parent_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        for folder in children.folders {
            *walk.count += 1;
            if *walk.count > MAX_PROPFIND_INFINITY_ITEMS {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }
            walk.xml.push_str(&collection_response(
                &current_segments,
                Some(&folder.name),
                walk.requested,
                Some(folder.updated_at),
                walk.lock_discoveries,
            ));
            if walk.recursive {
                let mut child_segments = current_segments.clone();
                child_segments.push(folder.name);
                pending.push((Some(folder.id), child_segments));
            }
        }
        for file in children.files {
            *walk.count += 1;
            if *walk.count > MAX_PROPFIND_INFINITY_ITEMS {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }
            walk.xml.push_str(&file_response(
                &current_segments,
                &file,
                walk.requested,
                walk.lock_discoveries,
            ));
        }
    }

    Ok(())
}

async fn propfind_file(
    state: &AppState,
    principal: &WebDavPrincipal,
    segments: &[String],
    requested: &PropfindRequest,
) -> Response {
    let Ok((folder_id, filename)) = WebDavService::new(state)
        .resolve_parent_and_name(principal, segments)
        .await
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(file) = WebDavService::new(state)
        .find_file(principal.user_id, folder_id, &filename)
        .await
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let lock_discoveries =
        if requested.props.contains(&DavProp::LockDiscovery) && !requested.propname {
            match active_lock_discoveries(state, principal.user_id).await {
                Ok(lock_discoveries) => lock_discoveries,
                Err(status) => return status.into_response(),
            }
        } else {
            HashMap::new()
        };
    let mut xml = multistatus_start();
    xml.push_str(&file_response(
        &segments[..segments.len() - 1],
        &file,
        requested,
        &lock_discoveries,
    ));
    xml.push_str(&xml_end("D:multistatus"));
    (
        StatusCode::MULTI_STATUS,
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DavProp {
    DisplayName,
    ResourceType,
    ContentLength,
    ContentType,
    Etag,
    LastModified,
    CreationDate,
    SupportedLock,
    LockDiscovery,
}

#[derive(Debug, Clone)]
struct PropfindRequest {
    props: Vec<DavProp>,
    unsupported: Vec<String>,
    propname: bool,
}

fn default_propfind_request() -> PropfindRequest {
    PropfindRequest {
        props: vec![
            DavProp::DisplayName,
            DavProp::ResourceType,
            DavProp::ContentLength,
            DavProp::ContentType,
            DavProp::Etag,
            DavProp::LastModified,
            DavProp::CreationDate,
            DavProp::SupportedLock,
            DavProp::LockDiscovery,
        ],
        unsupported: Vec::new(),
        propname: false,
    }
}

fn default_props() -> Vec<DavProp> {
    vec![
        DavProp::DisplayName,
        DavProp::ResourceType,
        DavProp::ContentLength,
        DavProp::ContentType,
        DavProp::Etag,
        DavProp::LastModified,
        DavProp::CreationDate,
        DavProp::SupportedLock,
        DavProp::LockDiscovery,
    ]
}

async fn requested_propfind_props(body: Body) -> Result<PropfindRequest, StatusCode> {
    let Ok(bytes) = to_bytes(body, 64 * 1024).await else {
        return Err(StatusCode::BAD_REQUEST);
    };
    if bytes.is_empty() {
        return Ok(default_propfind_request());
    }
    let xml = std::str::from_utf8(&bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    parse_propfind_request(xml).map_err(|_| StatusCode::BAD_REQUEST)
}

fn parse_propfind_request(xml: &str) -> Result<PropfindRequest, quick_xml::Error> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut stack: Vec<String> = Vec::new();
    let mut props = Vec::new();
    let mut unsupported = Vec::new();

    loop {
        match reader.read_event()? {
            Event::Start(event) => {
                let name = xml_local_name(event.local_name().as_ref());
                if name == "propname" {
                    return Ok(PropfindRequest {
                        props: default_props(),
                        unsupported: Vec::new(),
                        propname: true,
                    });
                }
                if name == "allprop" {
                    return Ok(default_propfind_request());
                }
                if stack.last().is_some_and(|parent| parent == "prop") {
                    collect_propfind_property(&name, &mut props, &mut unsupported);
                }
                stack.push(name);
            }
            Event::Empty(event) => {
                let name = xml_local_name(event.local_name().as_ref());
                if name == "propname" {
                    return Ok(PropfindRequest {
                        props: default_props(),
                        unsupported: Vec::new(),
                        propname: true,
                    });
                }
                if name == "allprop" {
                    return Ok(default_propfind_request());
                }
                if stack.last().is_some_and(|parent| parent == "prop") {
                    collect_propfind_property(&name, &mut props, &mut unsupported);
                }
            }
            Event::End(_) => {
                let _ = stack.pop();
            }
            Event::Eof => {
                if !stack.is_empty() {
                    return Err(quick_xml::Error::IllFormed(
                        quick_xml::errors::IllFormedError::MissingEndTag(
                            stack.last().cloned().unwrap_or_default(),
                        ),
                    ));
                }
                break;
            }
            _ => {}
        }
    }

    if props.is_empty() && unsupported.is_empty() {
        Ok(default_propfind_request())
    } else {
        Ok(PropfindRequest {
            props,
            unsupported,
            propname: false,
        })
    }
}

pub(super) fn xml_local_name(name: &[u8]) -> String {
    std::str::from_utf8(name)
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn collect_propfind_property(name: &str, props: &mut Vec<DavProp>, unsupported: &mut Vec<String>) {
    let prop = match name {
        "displayname" => Some(DavProp::DisplayName),
        "resourcetype" => Some(DavProp::ResourceType),
        "getcontentlength" => Some(DavProp::ContentLength),
        "getcontenttype" => Some(DavProp::ContentType),
        "getetag" => Some(DavProp::Etag),
        "getlastmodified" => Some(DavProp::LastModified),
        "creationdate" => Some(DavProp::CreationDate),
        "supportedlock" => Some(DavProp::SupportedLock),
        "lockdiscovery" => Some(DavProp::LockDiscovery),
        _ => None,
    };
    if let Some(prop) = prop {
        if !props.contains(&prop) {
            props.push(prop);
        }
        return;
    }
    if !matches!(name, "propfind" | "prop") && !unsupported.iter().any(|known| known == name) {
        unsupported.push(name.to_string());
    }
}

#[allow(dead_code)]
fn requested_xml_element_names(xml: &str) -> Vec<String> {
    let Ok(request) = parse_propfind_request(xml) else {
        return Vec::new();
    };
    request.unsupported
}

fn lock_path_for_child(segments: &[String], child: Option<&str>) -> String {
    let mut path_segments = segments.to_vec();
    if let Some(child) = child {
        path_segments.push(child.to_string());
    }
    lock_key(&path_segments)
}

fn collection_response(
    segments: &[String],
    child: Option<&str>,
    requested: &PropfindRequest,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    lock_discoveries: &LockDiscoveryByPath,
) -> String {
    let displayname = child
        .map(str::to_string)
        .or_else(|| segments.last().cloned())
        .unwrap_or_else(|| "dav".to_string());
    let mut props = String::new();
    if requested.props.contains(&DavProp::DisplayName) {
        if requested.propname {
            props.push_str(&xml_empty("D:displayname"));
        } else {
            props.push_str(&xml_text("D:displayname", &displayname));
        }
    }
    if requested.props.contains(&DavProp::ResourceType) {
        if requested.propname {
            props.push_str(&xml_empty("D:resourcetype"));
        } else {
            props.push_str(&xml_element("D:resourcetype", &xml_empty("D:collection")));
        }
    }
    if requested.props.contains(&DavProp::LastModified) {
        if requested.propname {
            props.push_str(&xml_empty("D:getlastmodified"));
        } else if let Some(updated_at) = updated_at {
            props.push_str(&xml_text("D:getlastmodified", &updated_at.to_rfc2822()));
        }
    }
    if requested.props.contains(&DavProp::SupportedLock) {
        if requested.propname {
            props.push_str(&xml_empty("D:supportedlock"));
        } else {
            props.push_str(&supported_lock_xml());
        }
    }
    if requested.props.contains(&DavProp::LockDiscovery) {
        if requested.propname {
            props.push_str(&xml_empty("D:lockdiscovery"));
        } else {
            let path = lock_path_for_child(segments, child);
            if let Some(lock_discovery) = lock_discoveries.get(&path) {
                props.push_str(lock_discovery);
            } else {
                props.push_str(&xml_empty("D:lockdiscovery"));
            }
        }
    }
    let unsupported = unsupported_propstat(&requested.unsupported);
    response_xml(&dav_href(segments, child, true), &props, &unsupported)
}

fn file_response(
    segments: &[String],
    file: &crate::models::file::File,
    requested: &PropfindRequest,
    lock_discoveries: &LockDiscoveryByPath,
) -> String {
    let mut props = String::new();
    if requested.props.contains(&DavProp::DisplayName) {
        if requested.propname {
            props.push_str(&xml_empty("D:displayname"));
        } else {
            props.push_str(&xml_text("D:displayname", &file.original_filename));
        }
    }
    if requested.props.contains(&DavProp::ResourceType) {
        props.push_str(&xml_empty("D:resourcetype"));
    }
    if requested.props.contains(&DavProp::ContentLength) {
        if requested.propname {
            props.push_str(&xml_empty("D:getcontentlength"));
        } else {
            props.push_str(&xml_text("D:getcontentlength", &file.file_size.to_string()));
        }
    }
    if requested.props.contains(&DavProp::ContentType) {
        if requested.propname {
            props.push_str(&xml_empty("D:getcontenttype"));
        } else {
            props.push_str(&xml_text("D:getcontenttype", &file.mime_type));
        }
    }
    if requested.props.contains(&DavProp::Etag) {
        if requested.propname {
            props.push_str(&xml_empty("D:getetag"));
        } else {
            props.push_str(&xml_text(
                "D:getetag",
                &format!("\"{}-{}\"", file.id, file.updated_at.timestamp()),
            ));
        }
    }
    if requested.props.contains(&DavProp::LastModified) {
        if requested.propname {
            props.push_str(&xml_empty("D:getlastmodified"));
        } else {
            props.push_str(&xml_text(
                "D:getlastmodified",
                &file.updated_at.to_rfc2822(),
            ));
        }
    }
    if requested.props.contains(&DavProp::CreationDate) {
        if requested.propname {
            props.push_str(&xml_empty("D:creationdate"));
        } else {
            props.push_str(&xml_text("D:creationdate", &file.created_at.to_rfc3339()));
        }
    }
    if requested.props.contains(&DavProp::SupportedLock) {
        if requested.propname {
            props.push_str(&xml_empty("D:supportedlock"));
        } else {
            props.push_str(&supported_lock_xml());
        }
    }
    if requested.props.contains(&DavProp::LockDiscovery) {
        if requested.propname {
            props.push_str(&xml_empty("D:lockdiscovery"));
        } else {
            let path = lock_path_for_child(segments, Some(&file.original_filename));
            if let Some(lock_discovery) = lock_discoveries.get(&path) {
                props.push_str(lock_discovery);
            } else {
                props.push_str(&xml_empty("D:lockdiscovery"));
            }
        }
    }
    let unsupported = unsupported_propstat(&requested.unsupported);
    response_xml(
        &dav_href(segments, Some(&file.original_filename), false),
        &props,
        &unsupported,
    )
}

fn unsupported_propstat(names: &[String]) -> String {
    if names.is_empty() {
        return String::new();
    }
    let props = names
        .iter()
        .map(|name| xml_empty(&format!("D:{name}")))
        .collect::<String>();
    propstat_xml(&props, "HTTP/1.1 404 Not Found")
}

fn response_xml(href: &str, props: &str, unsupported: &str) -> String {
    let mut body = xml_text("D:href", href);
    body.push_str(&propstat_xml(props, "HTTP/1.1 200 OK"));
    body.push_str(unsupported);
    xml_element("D:response", &body)
}

fn propstat_xml(props: &str, status: &str) -> String {
    let mut body = xml_element("D:prop", props);
    body.push_str(&xml_text("D:status", status));
    xml_element("D:propstat", &body)
}

fn supported_lock_xml() -> String {
    let lock_scope = xml_element("D:lockscope", &xml_empty("D:exclusive"));
    let lock_type = xml_element("D:locktype", &xml_empty("D:write"));
    xml_element(
        "D:supportedlock",
        &xml_element("D:lockentry", &(lock_scope + &lock_type)),
    )
}
