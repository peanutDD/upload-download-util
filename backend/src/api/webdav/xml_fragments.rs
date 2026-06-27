use quick_xml::{
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
    writer::Writer,
};

pub(super) fn multistatus_start() -> String {
    build_xml_fragment(|writer| {
        writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;
        let mut start = BytesStart::new("D:multistatus");
        start.push_attribute(("xmlns:D", "DAV:"));
        writer.write_event(Event::Start(start))?;
        Ok(())
    })
}

fn build_xml_fragment<F>(write: F) -> String
where
    F: FnOnce(&mut Writer<Vec<u8>>) -> Result<(), quick_xml::Error>,
{
    let mut writer = Writer::new(Vec::new());
    if write(&mut writer).is_err() {
        return String::new();
    }
    String::from_utf8(writer.into_inner()).unwrap_or_default()
}

pub(super) fn xml_start(name: &str) -> String {
    build_xml_fragment(|writer| {
        writer.write_event(Event::Start(BytesStart::new(name)))?;
        Ok(())
    })
}

pub(super) fn xml_end(name: &str) -> String {
    build_xml_fragment(|writer| {
        writer.write_event(Event::End(BytesEnd::new(name)))?;
        Ok(())
    })
}

pub(super) fn xml_empty(name: &str) -> String {
    build_xml_fragment(|writer| {
        writer.write_event(Event::Empty(BytesStart::new(name)))?;
        Ok(())
    })
}

pub(super) fn xml_text(name: &str, value: &str) -> String {
    build_xml_fragment(|writer| {
        writer
            .create_element(name)
            .write_text_content(BytesText::new(value))?;
        Ok(())
    })
}

pub(super) fn xml_element(name: &str, body: &str) -> String {
    let mut xml = xml_start(name);
    xml.push_str(body);
    xml.push_str(&xml_end(name));
    xml
}

pub(super) fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_text_escapes_content_and_multistatus_has_dav_namespace() {
        assert_eq!(
            xml_text("D:href", "/dav/a&b"),
            "<D:href>/dav/a&amp;b</D:href>"
        );
        assert!(multistatus_start().contains("xmlns:D=\"DAV:\""));
    }
}
